// [INPUT]: 身份 SQLite 事务、本软件随机身份 ID 与浏览器资料槽位。
// [OUTPUT]: 身份到独占浏览器资料的绑定、稳定非零种子、与身份恢复同步提交的选择指针。
// [POS]: 浏览器联动的持久元数据；不读取 Cookie，不保存网站凭据，不把账号 UUID 暴露给引擎。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
use super::{
    database::{error, Database, Record},
    valid_id,
};
use rusqlite::{params, OptionalExtension, Transaction};
use serde::Serialize;

#[derive(Serialize)]
pub struct BrowserLink {
    pub enabled: bool,
    pub bound_ids: Vec<String>,
}
fn get(tx: &Transaction<'_>, key: &str) -> Result<Option<String>, String> {
    tx.query_row("SELECT value FROM state WHERE key=?1", [key], |r| r.get(0))
        .optional()
        .map_err(error)
}
fn set(tx: &Transaction<'_>, key: &str, value: &str) -> Result<(), String> {
    tx.execute(
        "INSERT OR REPLACE INTO state VALUES(?1,?2)",
        params![key, value],
    )
    .map_err(error)?;
    Ok(())
}
pub fn valid_slot(slot: &str) -> bool {
    slot == "legacy" || valid_id(slot)
}
pub fn seed(id: &str, attempt: u32) -> u32 {
    let digest = ring::digest::digest(
        &ring::digest::SHA256,
        format!("CCBaga/browser/{id}/{attempt}").as_bytes(),
    );
    u32::from_be_bytes(digest.as_ref()[..4].try_into().unwrap())
}
fn bind(tx: &Transaction<'_>, id: &str, preferred: Option<&str>) -> Result<String, String> {
    if !valid_id(id) {
        return Err("身份编号无效。".into());
    }
    if let Some(slot) = tx
        .query_row(
            "SELECT slot FROM browser_bindings WHERE identity=?1",
            [id],
            |r| r.get::<_, String>(0),
        )
        .optional()
        .map_err(error)?
    {
        return Ok(slot);
    }
    let mut slot = preferred
        .filter(|s| valid_slot(s))
        .map(str::to_owned)
        .unwrap_or(super::new_id()?);
    if tx
        .query_row(
            "SELECT COUNT(*) FROM browser_bindings WHERE slot=?1",
            [&slot],
            |r| r.get::<_, i64>(0),
        )
        .map_err(error)?
        != 0
    {
        slot = super::new_id()?;
    }
    for attempt in 0..1024 {
        let seed = seed(id, attempt);
        if seed != 0
            && tx
                .query_row(
                    "SELECT COUNT(*) FROM browser_bindings WHERE seed=?1",
                    [seed],
                    |r| r.get::<_, i64>(0),
                )
                .map_err(error)?
                == 0
        {
            tx.execute(
                "INSERT INTO browser_bindings(identity,slot,seed) VALUES(?1,?2,?3)",
                params![id, slot, seed],
            )
            .map_err(error)?;
            return Ok(slot);
        }
    }
    Err("无法分配独立浏览器种子。".into())
}
pub(super) fn pending(tx: &Transaction<'_>, record: &Record, target: &str) -> Result<(), String> {
    if get(tx, "browser.enabled")?.as_deref() != Some("1") {
        return Ok(());
    }
    let previous = get(tx, "browser.active")?.ok_or("浏览器资料选择无效。")?;
    if !valid_slot(&previous) {
        return Err("浏览器资料选择无效。".into());
    }
    if !record.empty {
        bind(tx, &record.id, Some(&previous))?;
    }
    let next = if target == "empty" {
        super::new_id()?
    } else {
        bind(tx, target, None)?
    };
    set(tx, "browser.previous", &previous)?;
    set(tx, "browser.next", &next)
}
pub(super) fn complete(tx: &Transaction<'_>, active: &str) -> Result<(), String> {
    if let Some(previous) = get(tx, "browser.previous")? {
        let slot = if get(tx, "target")?.as_deref() == Some(active) {
            get(tx, "browser.next")?.ok_or("浏览器资料选择无效。")?
        } else {
            previous
        };
        if !valid_slot(&slot) {
            return Err("浏览器资料选择无效。".into());
        }
        set(tx, "browser.active", &slot)?;
        tx.execute(
            "DELETE FROM state WHERE key IN ('browser.previous','browser.next')",
            [],
        )
        .map_err(error)?;
    } else if active == "empty"
        && get(tx, "recovery")?.is_none()
        && get(tx, "browser.enabled")?.as_deref() == Some("1")
    {
        // 直接删除当前身份后，使用全新空资料；旧资料由独立删除选择决定。
        set(tx, "browser.active", &super::new_id()?)?;
    } else if valid_id(active)
        && get(tx, "recovery")?.is_none()
        && get(tx, "browser.enabled")?.as_deref() == Some("1")
    {
        // 当前 Claude 已是目标账号时仍对齐其浏览器绑定，不回灌旧登录归档。
        let previous = get(tx, "browser.active")?;
        let slot = bind(tx, active, previous.as_deref())?;
        set(tx, "browser.active", &slot)?;
    }
    Ok(())
}
pub(super) fn merge(tx: &Transaction<'_>, old: &str, retained: &str) -> Result<(), String> {
    let old_slot = tx
        .query_row(
            "SELECT slot FROM browser_bindings WHERE identity=?1",
            [old],
            |r| r.get::<_, String>(0),
        )
        .optional()
        .map_err(error)?;
    if let Some(slot) = old_slot {
        let retained_exists = tx
            .query_row(
                "SELECT COUNT(*) FROM browser_bindings WHERE identity=?1",
                [retained],
                |r| r.get::<_, i64>(0),
            )
            .map_err(error)?
            != 0;
        if !retained_exists || get(tx, "browser.active")?.as_deref() == Some(slot.as_str()) {
            // 合并旧身份卡时仍指向正在使用的资料；不删除被替代的资料目录。
            tx.execute(
                "DELETE FROM browser_bindings WHERE identity IN (?1,?2)",
                params![old, retained],
            )
            .map_err(error)?;
            bind(tx, retained, Some(&slot))?;
        }
    }
    Ok(())
}
impl Database {
    pub(super) fn browser_draft(&self) -> Result<Option<String>, String> {
        if !self.browser_link()?.enabled {
            return Ok(None);
        }
        let slot = self.browser_slot()?;
        if !valid_id(&slot) {
            return Ok(None);
        }
        let occupied:i64=self.connection.query_row("SELECT (SELECT COUNT(*) FROM identities WHERE id=?1)+(SELECT COUNT(*) FROM browser_bindings WHERE slot=?1 OR identity=?1)",[&slot],|r|r.get(0)).map_err(error)?;
        Ok((occupied == 0).then_some(slot))
    }
    pub fn browser_link(&self) -> Result<BrowserLink, String> {
        let mut query=self.connection.prepare("SELECT identity FROM browser_bindings WHERE identity IN (SELECT id FROM identities)").map_err(error)?;
        let ids = query
            .query_map([], |r| r.get(0))
            .map_err(error)?
            .collect::<Result<Vec<String>, _>>()
            .map_err(error)?;
        Ok(BrowserLink {
            enabled: self.get("browser.enabled")?.as_deref() == Some("1"),
            bound_ids: ids,
        })
    }
    pub fn browser_slot(&self) -> Result<String, String> {
        if self.get("browser.enabled")?.as_deref() != Some("1") {
            return Ok("legacy".into());
        }
        let slot = self.get("browser.active")?.ok_or("浏览器资料选择无效。")?;
        if !valid_slot(&slot) {
            return Err("浏览器资料选择无效。".into());
        }
        Ok(slot)
    }
    pub fn browser_seed(&self, slot: &str) -> Result<Option<u32>, String> {
        self.connection
            .query_row(
                "SELECT seed FROM browser_bindings WHERE slot=?1",
                [slot],
                |r| r.get(0),
            )
            .optional()
            .map_err(error)
    }
    pub fn browser_for(&self, id: &str) -> Result<Option<String>, String> {
        self.connection
            .query_row(
                "SELECT slot FROM browser_bindings WHERE identity=?1",
                [id],
                |r| r.get(0),
            )
            .optional()
            .map_err(error)
    }
    pub fn configure_browser(
        &mut self,
        enabled: bool,
        adopt: bool,
        current: Option<&str>,
    ) -> Result<(), String> {
        if self.get("recovery")?.is_some() {
            return Err("请先恢复未完成的身份切换。".into());
        }
        let tx = self.connection.transaction().map_err(error)?;
        let slot = if !enabled {
            "legacy".into()
        } else {
            let preferred = if adopt {
                "legacy".into()
            } else {
                super::new_id()?
            };
            match current {
                Some(id) => bind(&tx, id, Some(&preferred))?,
                None => preferred,
            }
        };
        set(&tx, "browser.enabled", if enabled { "1" } else { "0" })?;
        set(&tx, "browser.active", &slot)?;
        tx.commit().map_err(error)
    }
}
