import { useMemo, useState } from 'react'
import { api, formatSize, SkillView, StateSnapshot } from '../api'
import { AgentChip, Button, Empty, Modal } from './ui'
import PickAgents from './PickAgents'

type Notify = (msg: string, kind?: 'ok' | 'err') => void

export default function SkillsPage(props: {
  state: StateSnapshot
  reload: (silent?: boolean) => Promise<void>
  notify: Notify
  setBusy: (v: boolean) => void
  busy: boolean
}) {
  const { state, reload, notify, setBusy, busy } = props
  const [q, setQ] = useState('')
  const [src, setSrc] = useState<'all' | 'local' | 'github'>('all')
  const [sel, setSel] = useState<string[]>([])
  const [picking, setPicking] = useState<{ mode: 'deploy' | 'undeploy'; skills: SkillView[] } | null>(
    null,
  )
  const [deleting, setDeleting] = useState<SkillView | null>(null)

  const agents = useMemo(() => state.agents.filter((a) => a.enabled), [state.agents])

  const list = useMemo(() => {
    const kw = q.trim().toLowerCase()
    return state.skills.filter((s) => {
      if (src !== 'all' && s.source.kind !== src) return false
      if (!kw) return true
      return (
        s.name.toLowerCase().includes(kw) ||
        s.id.toLowerCase().includes(kw) ||
        s.description.toLowerCase().includes(kw) ||
        s.tags.some((t) => t.toLowerCase().includes(kw))
      )
    })
  }, [state.skills, q, src])

  const toggleSel = (id: string) =>
    setSel((prev) => (prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id]))

  const runOps = async (ops: { ok: boolean; message: string }[], verb: string) => {
    const ok = ops.filter((o) => o.ok).length
    const failed = ops.filter((o) => !o.ok)
    if (failed.length === 0) notify(`${verb}成功：${ok} 项`)
    else notify(`${verb}：成功 ${ok} 项，失败 ${failed.length} 项 —— ${failed[0].message}`, 'err')
    await reload(true)
  }

  const doDeploy = async (agentIds: string[]) => {
    const skills = picking?.skills ?? []
    setPicking(null)
    setBusy(true)
    try {
      const ops = await api.deploySelection(
        skills.map((s) => s.id),
        agentIds,
      )
      await runOps(ops, '部署')
      setSel([])
    } catch (e) {
      notify(String(e), 'err')
    } finally {
      setBusy(false)
    }
  }

  const doUndeploy = async (agentIds: string[]) => {
    const skills = picking?.skills ?? []
    setPicking(null)
    setBusy(true)
    try {
      const ops = await api.undeploySelection(
        skills.map((s) => s.id),
        agentIds,
      )
      await runOps(ops, '卸载')
      setSel([])
    } catch (e) {
      notify(String(e), 'err')
    } finally {
      setBusy(false)
    }
  }

  const doUpdate = async (s: SkillView) => {
    setBusy(true)
    try {
      const ops = await api.updateFromSource([s.id])
      await runOps(ops, '更新')
    } catch (e) {
      notify(String(e), 'err')
    } finally {
      setBusy(false)
    }
  }

  const doDelete = async (undeployAll: boolean) => {
    const s = deleting
    if (!s) return
    setDeleting(null)
    setBusy(true)
    try {
      await api.deleteSkill(s.id, undeployAll)
      notify(`已移除「${s.name}」`)
      await reload(true)
    } catch (e) {
      notify(String(e), 'err')
    } finally {
      setBusy(false)
    }
  }

  const selectedSkills = state.skills.filter((s) => sel.includes(s.id))

  return (
    <div>
      <div className="toolbar">
        <input
          className="input"
          style={{ width: 260 }}
          placeholder="搜索名称 / 描述 / 标签…"
          value={q}
          onChange={(e) => setQ(e.target.value)}
        />
        <div className="tabs">
          {(['all', 'local', 'github'] as const).map((k) => (
            <button key={k} className={src === k ? 'active' : ''} onClick={() => setSrc(k)}>
              {k === 'all' ? '全部' : k === 'local' ? '本地导入' : 'GitHub'}
            </button>
          ))}
        </div>
        <div className="spacer" />
        <Button size="sm" onClick={() => setSel(list.map((s) => s.id))}>
          全选当前
        </Button>
        <Button size="sm" onClick={() => setSel([])} disabled={sel.length === 0}>
          取消选择
        </Button>
      </div>

      {sel.length > 0 && (
        <div className="bulk-bar">
          <strong>已选 {sel.length} 个 Skill</strong>
          <div className="spacer" />
          <Button
            size="sm"
            variant="primary"
            disabled={busy}
            onClick={() => setPicking({ mode: 'deploy', skills: selectedSkills })}
          >
            一键部署到…
          </Button>
          <Button
            size="sm"
            variant="danger"
            disabled={busy}
            onClick={() => setPicking({ mode: 'undeploy', skills: selectedSkills })}
          >
            从…卸载
          </Button>
        </div>
      )}

      {list.length === 0 ? (
        <div className="card">
          <Empty
            icon="📦"
            title="还没有 Skill"
            hint="到「导入 Skill」页面，从本地目录或 GitHub 仓库添加第一个 Skill"
          />
        </div>
      ) : (
        <div className="grid">
          {list.map((s) => {
            const deployedAgents = agents.filter((a) => s.deployments[a.id]?.deployed)
            return (
              <div key={s.id} className={sel.includes(s.id) ? 'card skill-card selected' : 'card skill-card'}>
                <div className="skill-head">
                  <input
                    type="checkbox"
                    checked={sel.includes(s.id)}
                    onChange={() => toggleSel(s.id)}
                    style={{ marginTop: 3 }}
                  />
                  <div style={{ minWidth: 0, flex: 1 }}>
                    <div className="skill-title">{s.name}</div>
                    <div className="small muted mono truncate">{s.id}</div>
                  </div>
                  <span className={s.source.kind === 'github' ? 'chip info' : 'chip'}>
                    {s.source.kind === 'github'
                      ? `${s.source.owner}/${s.source.repo}`
                      : '本地目录'}
                  </span>
                </div>

                <div className="small" style={{ color: 'var(--text-2)' }}>
                  {s.description ? (
                    <span className="clamp-2">{s.description}</span>
                  ) : (
                    <span className="muted">（无描述）</span>
                  )}
                </div>

                <div className="deploy-chips">
                  {deployedAgents.length === 0 ? (
                    <span className="chip warn">未部署到任何 Agent</span>
                  ) : (
                    deployedAgents.map((a) => (
                      <AgentChip key={a.id} agent={a} info={s.deployments[a.id]} />
                    ))
                  )}
                </div>

                <div className="row wrap small muted">
                  <span>{s.fileCount} 个文件</span>
                  <span>·</span>
                  <span>{formatSize(s.sizeBytes)}</span>
                  {s.version && (
                    <>
                      <span>·</span>
                      <span>v{s.version}</span>
                    </>
                  )}
                  <div className="spacer" />
                  <span title={s.updatedAt}>{s.updatedAt.slice(5, 16)}</span>
                </div>

                <div className="row wrap" style={{ gap: 6 }}>
                  <Button
                    size="sm"
                    variant="primary"
                    disabled={busy}
                    onClick={() => setPicking({ mode: 'deploy', skills: [s] })}
                  >
                    部署到…
                  </Button>
                  <Button
                    size="sm"
                    disabled={busy || deployedAgents.length === 0}
                    onClick={() => setPicking({ mode: 'undeploy', skills: [s] })}
                  >
                    卸载
                  </Button>
                  {s.source.kind === 'github' && (
                    <Button size="sm" disabled={busy} onClick={() => doUpdate(s)}>
                      拉取更新
                    </Button>
                  )}
                  <div className="spacer" />
                  <Button size="sm" variant="ghost" onClick={() => api.revealPath(s.path)}>
                    目录
                  </Button>
                  <Button size="sm" variant="ghost" onClick={() => setDeleting(s)}>
                    删除
                  </Button>
                </div>
              </div>
            )
          })}
        </div>
      )}

      {picking && (
        <PickAgents
          state={state}
          mode={picking.mode}
          skills={picking.skills}
          onClose={() => setPicking(null)}
          onDone={(ids) => (picking.mode === 'deploy' ? doDeploy(ids) : doUndeploy(ids))}
        />
      )}

      {deleting && (
        <Modal
          title={`移除「${deleting.name}」`}
          onClose={() => setDeleting(null)}
          footer={
            <>
              <Button onClick={() => setDeleting(null)}>取消</Button>
              <Button onClick={() => doDelete(false)}>仅从仓库移除</Button>
              <Button variant="danger" onClick={() => doDelete(true)}>
                同时从所有 Agent 卸载
              </Button>
            </>
          }
        >
          <div>
            这会从 SkillBox 本地仓库删除该 Skill 的正本
            <span className="mono"> {deleting.path}</span>。
          </div>
          <div className="hint" style={{ marginTop: 8 }}>
            选择「同时从所有 Agent 卸载」会一并删除它在各 Agent 目录下的副本；否则副本会保留，但不再受 SkillBox 管理。
          </div>
        </Modal>
      )}
    </div>
  )
}
