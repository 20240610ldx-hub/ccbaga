/**
 * [INPUT]: 五项标准安装 agent/调用入口与精简中英文 YAML 提示词。
 * [OUTPUT]: npm 命令、调用符、Skill 来源、语言和恰好五项的回归。
 * [POS]: 网络页复制内容验收，不安装 Skill 或连接服务器。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import {test,expect} from 'bun:test';
import {agents,installationPrompt} from './network-skill';
test('五项工具只复制正确语言的安装与运行提示词',()=>{
    expect(agents.map(a=>a.id)).toEqual(['generic','codex','cursor','opencode','copilot']);
    for(const agent of agents){
        const zh=installationPrompt(agent.id,'zh'),en=installationPrompt(agent.id,'en');
        if(agent.cli){expect(zh).toContain(`--agent ${agent.cli}`);expect(en).toContain(`--agent ${agent.cli}`);}
        expect(en).toContain('liseami/CCBaga');
        expect(en).toContain('npx --yes skills add');
        expect(en).toContain('--global --copy --yes');
        expect(en).toContain('DO_NOT_TRACK=1');
        expect(en).toContain('--skill ccbaga-network');
        expect(en).toContain(agent.invoke);expect(zh).toContain(agent.invoke);
        expect(en).toContain('English');
        expect(/[\u3400-\u9fff]/.test(en)).toBe(false);
        expect(/[\u3400-\u9fff]/.test(zh)).toBe(true);
        expect(en).not.toContain('VPS');expect(en).not.toContain('TUN');
    }
});
