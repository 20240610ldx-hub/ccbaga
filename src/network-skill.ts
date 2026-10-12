/**
 * [INPUT]: LobeHub 五项品牌 SVG 与共享双语安装/调用模板。
 * [OUTPUT]: agents 选项和 installationPrompt；使用 npm skills 的标准安装命令与真实 agent 标识/调用入口；关闭安装统计。
 * [POS]: 网络页纯数据/文案边界；只描述完整 Skill 的安装与运行，具体收集/验证/部署协议归 Skill。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import generic from '../assets/ai-icons/generic.svg';
import codex from '../assets/ai-icons/codex.svg';
import cursor from '../assets/ai-icons/cursor.svg';
import opencode from '../assets/ai-icons/opencode.svg';
import copilot from '../assets/ai-icons/copilot.svg';
import {translate,type Locale} from './i18n';
export const agents=[
    {id:'generic',name:'General',icon:generic,cli:'',invoke:'ccbaga-network'},
    {id:'codex',name:'Codex',icon:codex,cli:'codex',invoke:'$ccbaga-network'},
    {id:'cursor',name:'Cursor',icon:cursor,cli:'cursor',invoke:'/ccbaga-network'},
    {id:'opencode',name:'OpenCode',icon:opencode,cli:'opencode',invoke:'/ccbaga-network'},
    {id:'copilot',name:'Copilot',icon:copilot,cli:'github-copilot',invoke:'/ccbaga-network'},
];
export function installationPrompt(id:string,locale:Locale):string {
    const agent=agents.find(item=>item.id===id)||agents[0];
    const command=`npx --yes skills add liseami/CCBaga --skill ccbaga-network --global --copy --yes${agent.cli ? ` --agent ${agent.cli}` : ''}`;
    return translate(agent.cli ? '请设置 DO_NOT_TRACK=1，执行 `{v0}`。安装后运行 {v1}，使用中文。' : '请设置 DO_NOT_TRACK=1，使用 `{v0}` 安装 Skill，自行指定当前工具的 --agent。安装后运行 {v1}，使用中文。',
        {v0:command,v1:agent.invoke},locale);
}
