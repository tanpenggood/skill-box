import { useEffect, useMemo, useState } from 'react'
import { api, StateSnapshot } from './api'
import SkillsPage from './components/SkillsPage'
import ImportPage from './components/ImportPage'
import AgentsPage from './components/AgentsPage'
import MatrixPage from './components/MatrixPage'
import SettingsPage from './components/SettingsPage'
import { Button } from './components/ui'

type Tab = 'skills' | 'import' | 'agents' | 'matrix' | 'settings'

const NAV: { key: Tab; icon: string; label: string }[] = [
  { key: 'skills', icon: '📦', label: '技能库' },
  { key: 'import', icon: '⬇️', label: '导入 Skill' },
  { key: 'agents', icon: '🤖', label: 'Agent 管理' },
  { key: 'matrix', icon: '🔀', label: '部署矩阵' },
  { key: 'settings', icon: '⚙️', label: '设置与日志' },
]

type Toast = { kind: 'ok' | 'err'; msg: string } | null

export default function App() {
  const [tab, setTab] = useState<Tab>('skills')
  const [state, setState] = useState<StateSnapshot | null>(null)
  const [loading, setLoading] = useState(true)
  const [busy, setBusy] = useState(false)
  const [toast, setToast] = useState<Toast>(null)

  const notify = (msg: string, kind: 'ok' | 'err' = 'ok') => {
    setToast({ kind, msg })
    window.setTimeout(() => setToast(null), 3200)
  }

  const reload = async (silent = false) => {
    try {
      if (!silent) setLoading(true)
      const s = await api.getState()
      setState(s)
      applyTheme(s.config.theme)
    } catch (e) {
      notify(String(e), 'err')
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    reload()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const enabledAgents = useMemo(
    () => (state?.agents ?? []).filter((a) => a.enabled),
    [state?.agents],
  )

  const stats = useMemo(() => {
    const skills = state?.skills ?? []
    let deployed = 0
    for (const s of skills) {
      for (const a of state?.agents ?? []) {
        if (s.deployments[a.id]?.deployed) deployed += 1
      }
    }
    return {
      skills: skills.length,
      agents: enabledAgents.length,
      deployed,
      outdated: skills.reduce(
        (n, s) =>
          n +
          Object.values(s.deployments).filter((d) => d.deployed && d.status === 'modified').length,
        0,
      ),
    }
  }, [state, enabledAgents])

  const syncAll = async () => {
    setBusy(true)
    try {
      const ops = await api.syncDeployed([])
      const ok = ops.filter((o) => o.ok).length
      notify(`已同步 ${ok} 项` + (ok < ops.length ? `，${ops.length - ok} 项失败` : ''))
      await reload(true)
    } catch (e) {
      notify(String(e), 'err')
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="app">
      <aside className="sidebar">
        <div className="brand">
          <div className="brand-logo">S</div>
          <div>
            <div className="brand-name">SkillBox</div>
            <div className="brand-sub">跨 Agent 的 Skill 管理器</div>
          </div>
        </div>
        {NAV.map((n) => (
          <button
            key={n.key}
            className={tab === n.key ? 'nav-item active' : 'nav-item'}
            onClick={() => setTab(n.key)}
          >
            <span className="ico">{n.icon}</span>
            {n.label}
            {n.key === 'skills' && <span className="nav-badge">{stats.skills}</span>}
            {n.key === 'agents' && <span className="nav-badge">{stats.agents}</span>}
          </button>
        ))}
        <div className="sidebar-foot">
          <div>已部署 {stats.deployed} 处</div>
          <div>{stats.outdated > 0 ? `${stats.outdated} 处待同步` : '全部已同步'}</div>
        </div>
      </aside>

      <div className="main">
        <header className="topbar">
          <div>
            <h1>{NAV.find((n) => n.key === tab)?.label}</h1>
          </div>
          <span className="sub">
            {state
              ? `本地仓库 ${state.config.storeDir || state.home} · ${state.platform}`
              : '加载中…'}
          </span>
          <div className="spacer" />
          <Button size="sm" onClick={() => reload(true)} disabled={loading || busy}>
            {loading ? '刷新中…' : '刷新'}
          </Button>
          <Button size="sm" variant="primary" onClick={syncAll} disabled={busy || !state}>
            一键同步全部
          </Button>
        </header>

        <div className="content">
          {!state ? (
            <div className="empty">正在读取本地仓库…</div>
          ) : tab === 'skills' ? (
            <SkillsPage state={state} reload={reload} notify={notify} setBusy={setBusy} busy={busy} />
          ) : tab === 'import' ? (
            <ImportPage state={state} reload={reload} notify={notify} setBusy={setBusy} />
          ) : tab === 'agents' ? (
            <AgentsPage state={state} reload={reload} notify={notify} />
          ) : tab === 'matrix' ? (
            <MatrixPage state={state} reload={reload} notify={notify} setBusy={setBusy} busy={busy} />
          ) : (
            <SettingsPage state={state} reload={reload} notify={notify} />
          )}
        </div>
      </div>

      {toast && <div className={toast.kind === 'err' ? 'toast err' : 'toast'}>{toast.msg}</div>}
    </div>
  )
}

function applyTheme(theme: string) {
  const t = theme === 'dark' ? 'dark' : 'light'
  document.documentElement.setAttribute('data-theme', t)
}
