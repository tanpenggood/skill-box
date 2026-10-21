import { useEffect, useState } from 'react'
import { api, LogEntry, pickFolder, StateSnapshot } from '../api'
import { Button, Switch } from './ui'

type Notify = (msg: string, kind?: 'ok' | 'err') => void

export default function SettingsPage(props: {
  state: StateSnapshot
  reload: (silent?: boolean) => Promise<void>
  notify: Notify
}) {
  const { state, reload, notify } = props
  const [logs, setLogs] = useState<LogEntry[]>([])
  const [storeDraft, setStoreDraft] = useState(state.config.storeDir)

  useEffect(() => {
    setStoreDraft(state.config.storeDir)
  }, [state.config.storeDir])

  useEffect(() => {
    api
      .getLogs()
      .then(setLogs)
      .catch(() => setLogs([]))
  }, [state])

  const save = async (patch: Partial<StateSnapshot['config']>, msg: string) => {
    try {
      await api.saveConfig({ ...state.config, ...patch })
      notify(msg)
      await reload(true)
    } catch (e) {
      notify(String(e), 'err')
    }
  }

  const changeStore = async () => {
    const p = storeDraft.trim()
    if (!p) {
      notify('仓库目录不能为空', 'err')
      return
    }
    await save({ storeDir: p }, '仓库目录已更新（下次刷新生效）')
  }

  return (
    <div style={{ maxWidth: 760 }}>
      <div className="card" style={{ padding: 16, marginBottom: 14 }}>
        <strong>本地仓库</strong>
        <div className="hint" style={{ margin: '4px 0 12px' }}>
          所有导入的 Skill 都会保留一份正本在这里，部署到各 Agent 时从这里复制。
        </div>
        <div className="row">
          <input
            className="input mono"
            style={{ flex: 1 }}
            value={storeDraft}
            onChange={(e) => setStoreDraft(e.target.value)}
          />
          <Button
            onClick={async () => {
              const p = await pickFolder(storeDraft || state.home)
              if (p) setStoreDraft(p)
            }}
          >
            浏览…
          </Button>
          <Button variant="primary" onClick={changeStore}>
            保存
          </Button>
          <Button onClick={() => api.revealPath(state.config.storeDir)}>打开</Button>
        </div>
      </div>

      <div className="card" style={{ padding: 16, marginBottom: 14 }}>
        <strong>部署行为</strong>

        <div className="row" style={{ marginTop: 12 }}>
          <div style={{ flex: 1 }}>
            <div>目标已存在时直接覆盖</div>
            <div className="hint">关闭后，若 Agent 目录中已存在同名 Skill 会跳过而不覆盖。</div>
          </div>
          <Switch
            on={state.config.overwriteOnDeploy}
            onChange={(v) => save({ overwriteOnDeploy: v }, '已保存')}
          />
        </div>

        <div className="row" style={{ marginTop: 14 }}>
          <div style={{ flex: 1 }}>
            <div>启动时自动同步</div>
            <div className="hint">打开应用时，把所有已部署的副本刷新为仓库中的最新内容。</div>
          </div>
          <Switch on={state.config.autoSync} onChange={(v) => save({ autoSync: v }, '已保存')} />
        </div>

        <div className="row" style={{ marginTop: 14 }}>
          <div style={{ flex: 1 }}>
            <div>外观</div>
            <div className="hint">浅色 / 深色主题</div>
          </div>
          <select
            className="select"
            value={state.config.theme}
            onChange={(e) => save({ theme: e.target.value }, '主题已切换')}
          >
            <option value="light">浅色</option>
            <option value="dark">深色</option>
          </select>
        </div>
      </div>

      <div className="card" style={{ padding: 16, marginBottom: 14 }}>
        <div className="row">
          <strong>关于</strong>
          <div className="spacer" />
          <span className="chip">v0.1.0</span>
          <span className="chip">{state.platform}</span>
        </div>
        <div className="hint" style={{ marginTop: 8 }}>
          SkillBox 用一份代码构建 Windows / macOS / Linux 桌面应用（Tauri v2 + React）。
          Skill 在各 Agent 之间以复制方式分发，互不干扰，也随时可以单独卸载。
        </div>
      </div>

      <div className="card" style={{ padding: 16 }}>
        <div className="row" style={{ marginBottom: 10 }}>
          <strong>操作日志</strong>
          <div className="spacer" />
          <Button
            size="sm"
            onClick={() => api.getLogs().then(setLogs).catch(() => setLogs([]))}
          >
            刷新
          </Button>
        </div>
        <div className="log-list">
          {logs.length === 0 ? (
            <div className="muted">暂无日志</div>
          ) : (
            logs.map((l, i) => (
              <div key={i}>
                <span className="muted">{l.time}</span>{' '}
                <span style={{ color: l.level === 'error' ? 'var(--danger)' : 'var(--primary)' }}>
                  [{l.level}]
                </span>{' '}
                {l.message}
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  )
}
