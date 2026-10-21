import { useMemo, useState } from 'react'
import { api, StateSnapshot } from '../api'
import { Button, Empty } from './ui'

type Notify = (msg: string, kind?: 'ok' | 'err') => void
type Pending = Record<string, 'add' | 'del'>

export default function MatrixPage(props: {
  state: StateSnapshot
  reload: (silent?: boolean) => Promise<void>
  notify: Notify
  setBusy: (v: boolean) => void
  busy: boolean
}) {
  const { state, reload, notify, setBusy, busy } = props
  const [q, setQ] = useState('')
  const [pending, setPending] = useState<Pending>({})

  const agents = useMemo(() => state.agents.filter((a) => a.enabled), [state.agents])

  const rows = useMemo(() => {
    const kw = q.trim().toLowerCase()
    return state.skills.filter(
      (s) =>
        !kw ||
        s.name.toLowerCase().includes(kw) ||
        s.id.toLowerCase().includes(kw),
    )
  }, [state.skills, q])

  const key = (skillId: string, agentId: string) => `${skillId}|${agentId}`

  const toggle = (skillId: string, agentId: string) => {
    const k = key(skillId, agentId)
    const deployed = state.skills.find((s) => s.id === skillId)?.deployments[agentId]?.deployed ?? false
    setPending((prev) => {
      const n = { ...prev }
      if (n[k]) delete n[k]
      else n[k] = deployed ? 'del' : 'add'
      return n
    })
  }

  const selectAllTo = (agentId: string) => {
    setPending((prev) => {
      const n = { ...prev }
      for (const s of rows) {
        const k = key(s.id, agentId)
        if (s.deployments[agentId]?.deployed) delete n[k]
        else n[k] = 'add'
      }
      return n
    })
  }

  const apply = async () => {
    const adds: Record<string, string[]> = {}
    const dels: Record<string, string[]> = {}
    for (const [k, act] of Object.entries(pending)) {
      const [sid, aid] = k.split('|')
      const bucket = act === 'add' ? adds : dels
      bucket[sid] = bucket[sid] ?? []
      bucket[sid].push(aid)
    }
    setBusy(true)
    try {
      let ok = 0
      let fail = 0
      let firstErr = ''
      for (const [sid, aids] of Object.entries(adds)) {
        const ops = await api.deploySelection([sid], aids)
        ok += ops.filter((o) => o.ok).length
        const bad = ops.filter((o) => !o.ok)
        fail += bad.length
        if (bad.length && !firstErr) firstErr = bad[0].message
      }
      for (const [sid, aids] of Object.entries(dels)) {
        const ops = await api.undeploySelection([sid], aids)
        ok += ops.filter((o) => o.ok).length
        const bad = ops.filter((o) => !o.ok)
        fail += bad.length
        if (bad.length && !firstErr) firstErr = bad[0].message
      }
      notify(
        fail === 0 ? `已应用 ${ok} 项变更` : `成功 ${ok} 项，失败 ${fail} 项 —— ${firstErr}`,
        fail === 0 ? 'ok' : 'err',
      )
      setPending({})
      await reload(true)
    } catch (e) {
      notify(String(e), 'err')
    } finally {
      setBusy(false)
    }
  }

  const count = Object.keys(pending).length

  if (state.skills.length === 0 || agents.length === 0) {
    return (
      <div className="card">
        <Empty icon="🔀" title="还没有可管理的组合" hint="先导入 Skill，并确保至少启用了一个 Agent" />
      </div>
    )
  }

  return (
    <div>
      <div className="toolbar">
        <input
          className="input"
          style={{ width: 240 }}
          placeholder="过滤 Skill…"
          value={q}
          onChange={(e) => setQ(e.target.value)}
        />
        <span className="chip">{rows.length} 个 Skill</span>
        <div className="spacer" />
        <Button size="sm" onClick={() => setPending({})} disabled={count === 0}>
          撤销变更 ({count})
        </Button>
        <Button size="sm" variant="primary" onClick={apply} disabled={busy || count === 0}>
          应用 {count} 项变更
        </Button>
      </div>

      <div className="table-wrap">
        <table className="matrix">
          <thead>
            <tr>
              <th style={{ minWidth: 220 }}>Skill</th>
              {agents.map((a) => (
                <th key={a.id} title={a.path}>
                  <div className="row" style={{ gap: 6 }}>
                    <span className="dot" style={{ background: a.color }} />
                    {a.name}
                  </div>
                  <button
                    className="btn ghost sm"
                    style={{ padding: '1px 6px', marginTop: 3 }}
                    onClick={() => selectAllTo(a.id)}
                  >
                    全选部署
                  </button>
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {rows.map((s) => (
              <tr key={s.id}>
                <td className="name-cell">
                  <div style={{ fontWeight: 600 }}>{s.name}</div>
                  <div className="small muted mono truncate">{s.id}</div>
                </td>
                {agents.map((a) => {
                  const info = s.deployments[a.id]
                  const p = pending[key(s.id, a.id)]
                  const cls = p === 'add'
                    ? 'cell-btn pending-add'
                    : p === 'del'
                      ? 'cell-btn pending-del'
                      : info?.deployed
                        ? 'cell-btn on'
                        : 'cell-btn'
                  const text = p === 'add' ? '待部署' : p === 'del' ? '待卸载' : info?.deployed
                    ? info.status === 'modified' ? '已改动' : '已部署'
                    : '未部署'
                  return (
                    <td key={a.id}>
                      <button className={cls} onClick={() => toggle(s.id, a.id)}>
                        {text}
                      </button>
                    </td>
                  )
                })}
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <div className="hint" style={{ marginTop: 10 }}>
        点击单元格切换部署状态，先在本地记录变更，确认无误后点「应用变更」一次性写入磁盘。
      </div>
    </div>
  )
}
