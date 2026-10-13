/**
 * [INPUT]: 共享双语目录和应用各页面的实际操作边界。
 * [OUTPUT]: 可从侧栏打开的使用说明、安全提示、账号风险与申诉指引。
 * [POS]: CCBaga-A 的帮助页；只解释本地行为和官方公开规则，不诊断或改变服务器账号状态。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { useI18n } from './i18n';

function List({ items }: { items: string[] }) {
    return <ul className="help-list">{items.map(item => <li key={item}>{item}</li>)}</ul>;
}

export function HelpPanel() {
    const { t } = useI18n();
    return <section className="system-page help-page">
        <div className="cleanup-title"><h1>{t('使用说明')}</h1></div>
        <div className="help-lede">
            <strong>{t('先阅读这里，再执行会结束进程、删除数据或修改系统的操作。')}</strong>
            <p>{t('CCBaga 管理的是本机环境和本地 Claude 数据；它不是 Anthropic 官方客户端，不能读取、修改或保证服务器端账号状态。')}</p>
        </div>
        <div className="help-grid">
            <article className="help-card">
                <h2>{t('傻瓜设置')}</h2>
                <List items={[
                    t('地区选择只准备本机和浏览器的语言、时区与区域设置，不会改变公网 IP，也不会让不受支持的地区变为受支持。'),
                    t('清理浏览器缓存会保留登录、书签和密码；“清空站点数据”会删除独立浏览器的 Cookie、站点存储与恢复会话，且不可撤销。'),
                    t('切换浏览器指纹会关闭并重新启动 CCBaga-B，请先保存未完成的浏览器工作。'),
                    t('暂存 Claude 身份会先停止已识别的 Claude 进程；未保存的任务可能被中断。')
                ]} />
            </article>
            <article className="help-card">
                <h2>{t('Claude 身份管理')}</h2>
                <List items={[
                    t('身份操作只处理本机白名单目录；项目源代码不会被扫描，服务器端会话和账号记录不会因此删除。'),
                    t('登录与身份数据始终随身份暂存；Skills、插件、MCP、个人设置、项目记录、会话历史和缓存可分别选择保留或暂存。'),
                    t('空身份适合在同一台电脑上开始新的本地工作区，但不等于隐藏设备，也不等于创建了新的服务器账号。'),
                    t('直接删除和删除暂存身份不可撤销；确认前请先导出或备份仍需要的本地资料。')
                ]} />
            </article>
            <article className="help-card">
                <h2>{t('浏览器与电脑环境')}</h2>
                <List items={[
                    t('浏览器实测页只报告它实际观测到的出口、语言、时区、平台与设备信息；绿色结果不保证匿名、隐私或账号接受。'),
                    t('浏览器代理、TUN、系统代理和机器出口是不同层次；页面检测到的结果不能替代对代理工具规则的逐域核验。'),
                    t('对齐电脑环境会修改本机时区、首选语言、区域或定位权限，可能需要管理员授权、重启或注销；它不会修改代理节点。')
                ]} />
            </article>
            <article className="help-card">
                <h2>{t('网络与 Skill')}</h2>
                <List items={[
                    t('网络页只提供固定资源入口和 Skill 安装提示词；应用本身不收集 SSH 密钥或代理凭据，也不执行远程部署。'),
                    t('把提示词交给你选择的 AI 工具前，请检查文件路径、服务器地址、密钥和即将执行的命令；远端变更应由你明确批准。'),
                    t('代理或 VPS 只改变网络路径，不改变 Anthropic 的支持地区、服务条款或使用政策。')
                ]} />
            </article>
        </div>
        <article className="help-card help-risk" role="note">
            <h2>{t('Claude 账号风险与封禁说明')}</h2>
            <p>{t('使用 CCBaga 不能保证 Claude 账号不被警告、限流、暂停或终止。Anthropic 的公开规则和帮助中心允许其根据 Usage Policy、Consumer Terms 和 Supported Regions Policy 采取这些措施；项目无法预测风控结果，也不代表 Anthropic。')}</p>
            <List items={[
                t('公开规则明确禁止共享账号或凭据、未经授权的代理或转售、自动化注册或垃圾行为、为规避检测而使用多个账号、绕过封禁，以及绕过登录或安全限制。'),
                t('支持地区由 Anthropic 单独定义；本机切换语言、时区、指纹、代理或出口，不会把不受支持的实体或所在地变成受支持。'),
                t('X 和其他社区中的封禁案例多为个人叙述，通常缺少完整的地区、支付、账号和工具链证据；它们不能证明某个 IP、关键词、指纹或 CCBaga 本身导致封禁。')
            ]} />
        </article>
        <article className="help-card">
            <h2>{t('如果账号被限制')}</h2>
            <List items={[
                t('先保存页面提示、时间、登录方式和相关工单，不要把密码、Cookie、令牌或完整支付资料交给第三方。'),
                t('按 Anthropic 帮助中心指引，用受影响的账号登录 claude.ai，在受限页面提交申诉；符合条件时也可从同一页面导出数据或删除账号。'),
                t('不要通过新账号、共享账号或第三方中转规避封禁；申诉是否通过、处理多久和可导出哪些数据都由 Anthropic 决定。')
            ]} />
            <p className="help-source-note">{t('官方依据：Anthropic Usage Policy、Supported countries and regions、Safeguards warnings and appeals。仓库版来源与更新时间见 docs/usage.md。')}</p>
        </article>
    </section>;
}
