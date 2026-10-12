/**
 * [INPUT]: 三个 AI 品牌 SVG 与共享 YAML 提示词模板。
 * [OUTPUT]: agents 选项和 installationPrompt；按所选工具指定真实个人 Skill 目录。
 * [POS]: 网络页纯数据/文案边界；复制完整 Skill，安装授权不扩展为购买或远端部署授权。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import claude from '../assets/ai-icons/claude.svg';
import codex from '../assets/ai-icons/openai.svg';
import cursor from '../assets/ai-icons/cursor.svg';
import {translate,type Locale} from './i18n';
export const agents=[
    {id:'claude',name:'Claude Code',icon:claude,directory:'~/.claude/skills/ccbaga-network'},
    {id:'codex',name:'Codex',icon:codex,directory:'~/.agents/skills/ccbaga-network'},
    {id:'cursor',name:'Cursor',icon:cursor,directory:'~/.cursor/skills/ccbaga-network'},
];
export function installationPrompt(id:string,locale:Locale):string {
    const agent=agents.find(item=>item.id===id)||agents[0];
    return translate('请帮我安装并使用 CCBaga 网络 Skill。官方仓库：{v0}；完整目录：skills/ccbaga-network；安装到 {v1}，供 {v2} 使用。先检查脚本、保留已有私密配置并运行离线测试。安装期间不要购买、连接 VPS 或部署。安装好后用中文向我要一份资料：VPS 地址、PEM/PPK/PUB 三个文件（可直接附上密钥包）、家宽地址、端口、账号和密码（可直接提供供应商连接资料）。缺什么再告诉我去哪里获取。配置文件、终端命令、本机验证、经我确认的部署和订阅生成都由你操作，不要让我打开终端、输入命令或编辑 JSON。最后带我开启 TUN、关闭系统代理并验证出口。我的凭据只用于此次配置，不写入公开日志或仓库。',
        {v0:'https://github.com/liseami/CCBaga',v1:agent.directory,v2:agent.name},locale);
}
