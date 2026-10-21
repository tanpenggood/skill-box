import { useState } from 'react'
import { api, pickFolder, StateSnapshot } from '../api'
import { Button, Switch } from './ui'

type Notify = (msg: string, kind?: 'ok' | 'err') => void

export default function AgentsPage(props: {
  state: StateSnapshot
  reload: (silent?: boolean) => Promise<void>
  notify: Notify
}) {
  const { state, reload, notify } = props
  const [editing, setEditing] = useState<string | null>(null)
  const [draft, setDraft] = useState('')
  const [newName, setNewName] = useState('')
  const [newPath, setNewPath] = useState('')

  const act = async (fn: () => Promise<unknown>, okMsg: string) => {
    try {
      await fn()
      notify(okMsg)
      await reload(true)
    } catch (e) {
      notify(String(e), 'err')
    }
  }

  const savePath = async (id: string) => {
    await act(() => api.setAgentPath(id, draft.trim()), '目录已更新')
    setEditing(null)
  }

  const addAgent = async () => {
    if (!newName.trim() || !newPath.trim()) {
      notify('请填写名称和 skills 目录', 'err')
      return
    }
    await act(() => api.addCustomAgent(newName.trim(), newPath.trim()), `已添加 ${newName.trim()}`)
    setNewName('')
    setNewPath('')
  }

  const detected = state.agents.filter((a) => a.installed).length

  return (
    <div>
      <div className="card" style={{ marginBottom: 14 }}>
        <div className="agent-row" style={{ background: 'var(--panel-2)' }}>
          <div>
            <strong>Agent 目标</strong>
            <div className="small muted">
              共 {state.agents.length} 个，本机检测到 {detected} 个。禁用的 Agent 不会出现在部署选项中。
            </div>
          </div>
          <div className="spacer" />
        </div>

        {state.agents.map((a) => (
          <div key={a.id} className="agent-row">
            <span className="avatar" style={{ background: a.color }}>
              {a.short}
            </span>
            <div style={{ minWidth: 0, flex: 1 }}>
              <div className="row">
                <strong>{a.name}</strong>
                <span className="chip">{a.kind === 'builtin' ? '内置' : '自定义'}</span>
                {a.installed ? (
                  a.ready ? (
                    <span className="chip ok">已就绪</span>
                  ) : (
                    <span className="chip warn">已安装 · skills 目录未创建</span>
                  )
                ) : (
                  <span className="chip">未检测到</span>
                )}
                {!a.writable && <span className="chip bad">不可写</span>}
              </div>
              {editing === a.id ? (
                <div className="row" style={{ marginTop: 6 }}>
                  <input
                    className="input mono"
                    style={{ flex: 1 }}
                    value={draft}
                    onChange={(e) => setDraft(e.target.value)}
                  />
                  <Button size="sm" variant="primary" onClick={() => savePath(a.id)}>
                    保存
                  </Button>
                  <Button size="sm" onClick={() => setEditing(null)}>
                    取消
                  </Button>
                  <Button
                    size="sm"
                    onClick={async () => {
                      const p = await pickFolder(a.path)
                      if (p) setDraft(p)
                    }}
                  >
                    浏览
                  </Button>
                </div>
              ) : (
                <div className="row" style={{ marginTop: 2 }}>
                  <span className="small muted mono truncate" title={a.path}>
                    {a.path}
                  </span>
                  <Button size="sm" variant="ghost" onClick={() => { setEditing(a.id); setDraft(a.path) }}>
                    修改
                  </Button>
                </div>
              )}
              <div className="small muted" style={{ marginTop: 2 }}>
                目录内 {a.skillCount} 个 skill，其中 {a.managedCount} 个由 SkillBox 管理
              </div>
            </div>

            <div className="row">
              {a.homepage && (
                <Button size="sm" variant="ghost" title={a.homepage} onClick={() => api.revealPath(a.path)}>
                  打开目录
                </Button>
              )}
              {!a.homepage && (
                <Button size="sm" variant="ghost" onClick={() => api.revealPath(a.path)}>
                  打开目录
                </Button>
              )}
              {a.kind === 'custom' && (
                <Button
                  size="sm"
                  variant="danger"
                  onClick={() => act(() => api.removeCustomAgent(a.id), `已移除 ${a.name}`)}
                >
                  移除
                </Button>
              )}
              <Switch
                on={a.enabled}
                onChange={(v) => act(() => api.toggleAgent(a.id, v), v ? '已启用' : '已禁用')}
              />
            </div>
          </div>
        ))}
      </div>

      <div className="card" style={{ padding: 16 }}>
        <strong>添加自定义 Agent</strong>
        <div className="hint" style={{ margin: '4px 0 12px' }}>
          如果你的工具不在内置列表里，直接指定它的 skills 目录即可（例如项目级
          <span className="mono"> .claude/skills </span>）。
        </div>
        <div className="row">
          <input
            className="input"
            style={{ width: 200 }}
            placeholder="名称，例如 Augment"
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
          />
          <input
            className="input mono"
            style={{ flex: 1 }}
            placeholder="skills 目录绝对路径"
            value={newPath}
            onChange={(e) => setNewPath(e.target.value)}
          />
          <Button
            onClick={async () => {
              const p = await pickFolder(state.home)
              if (p) setNewPath(p)
            }}
          >
            浏览…
          </Button>
          <Button variant="primary" onClick={addAgent}>
            添加
          </Button>
        </div>
      </div>
    </div>
  )
}
