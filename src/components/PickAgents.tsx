import { useEffect, useMemo, useState } from 'react'
import { AgentView, SkillView, StateSnapshot } from '../api'
import { Modal, Button } from './ui'

export default function PickAgents(props: {
  state: StateSnapshot
  mode: 'deploy' | 'undeploy'
  skills: SkillView[]
  onClose: () => void
  onDone: (agentIds: string[]) => void
}) {
  const { state, mode, skills } = props

  const candidates = useMemo(() => {
    if (mode === 'deploy') return state.agents.filter((a) => a.enabled)
    return state.agents.filter((a) => skills.some((s) => s.deployments[a.id]?.deployed))
  }, [state.agents, mode, skills])

  const defaultOn = useMemo(() => {
    const set = new Set<string>()
    for (const a of candidates) {
      if (mode === 'deploy') {
        // 默认勾选：尚未部署 或 内容已改动
        if (skills.some((s) => !s.deployments[a.id]?.deployed || s.deployments[a.id]?.status === 'modified'))
          set.add(a.id)
      } else {
        if (skills.some((s) => s.deployments[a.id]?.deployed)) set.add(a.id)
      }
    }
    return set
  }, [candidates, mode, skills])

  const [picked, setPicked] = useState<Set<string>>(new Set())

  useEffect(() => {
    setPicked(new Set(defaultOn))
  }, [defaultOn])

  const toggle = (id: string) =>
    setPicked((prev) => {
      const n = new Set(prev)
      n.has(id) ? n.delete(id) : n.add(id)
      return n
    })

  const ready = candidates.filter((a) => a.ready || a.installed)
  const others = candidates.filter((a) => !(a.ready || a.installed))

  const renderAgent = (a: AgentView) => {
    const count = skills.filter((s) => s.deployments[a.id]?.deployed).length
    return (
      <div
        key={a.id}
        className={picked.has(a.id) ? 'list-item checked' : 'list-item'}
        onClick={() => toggle(a.id)}
      >
        <input type="checkbox" checked={picked.has(a.id)} onChange={() => toggle(a.id)} />
        <span className="avatar" style={{ background: a.color, width: 28, height: 28, fontSize: 11 }}>
          {a.short}
        </span>
        <div style={{ minWidth: 0, flex: 1 }}>
          <div style={{ fontWeight: 600 }}>{a.name}</div>
          <div className="small muted mono truncate">{a.path}</div>
        </div>
        <div className="row small" style={{ gap: 6 }}>
          {!a.writable && <span className="chip bad">不可写</span>}
          {a.ready ? (
            <span className="chip">{a.skillCount} 个</span>
          ) : (
            <span className="chip warn">目录待创建</span>
          )}
          {mode === 'undeploy' && <span className="chip info">已装 {count}</span>}
        </div>
      </div>
    )
  }

  return (
    <Modal
      title={`${mode === 'deploy' ? '部署到' : '从以下 Agent 卸载'} · 已选 ${skills.length} 个 Skill`}
      onClose={props.onClose}
      footer={
        <>
          <Button onClick={props.onClose}>取消</Button>
          <Button
            variant={mode === 'deploy' ? 'primary' : 'danger'}
            disabled={picked.size === 0}
            onClick={() => props.onDone(Array.from(picked))}
          >
            {mode === 'deploy' ? `部署 (${picked.size})` : `卸载 (${picked.size})`}
          </Button>
        </>
      }
    >
      {candidates.length === 0 ? (
        <div className="empty">没有可操作的 Agent</div>
      ) : (
        <>
          {ready.map(renderAgent)}
          {others.length > 0 && (
            <>
              <div className="hint" style={{ margin: '10px 0 8px' }}>
                以下 Agent 尚未在本机检测到，SkillBox 会在部署时自动创建其 skills 目录：
              </div>
              {others.map(renderAgent)}
            </>
          )}
        </>
      )}
    </Modal>
  )
}
