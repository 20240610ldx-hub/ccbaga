/**
 * [INPUT]: 真实 AI 目录表、完整安装提示词和共享中英文 YAML。
 * [OUTPUT]: 三种工具对应目录、源仓库、语言切换及安装授权范围的离线回归。
 * [POS]: 网络页复制内容的行为验收；不安装 Skill 或连接服务器。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import {test,expect} from 'bun:test';
import {agents,installationPrompt} from './network-skill';
test('三个 AI 工具的提示词保留正确目录且完整切换英文',()=>{
    for(const agent of agents){
        const zh=installationPrompt(agent.id,'zh'),en=installationPrompt(agent.id,'en');
        expect(zh).toContain(agent.directory);expect(en).toContain(agent.directory);
        expect(en).toContain(agent.name);expect(en).toContain('https://github.com/liseami/CCBaga');
        expect(en).toContain('skills/ccbaga-network');
        expect(en).toContain('Do not purchase');
        expect(en).toContain('English');
        expect(en).toContain('Do not ask me to open a terminal');
        expect(zh).toContain('不要让我打开终端');
        expect(/[\u3400-\u9fff]/.test(en)).toBe(false);
        expect(/[\u3400-\u9fff]/.test(zh)).toBe(true);
    }
});
