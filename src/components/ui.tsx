import { ReactNode } from 'react'

export function Button(props: {
  children: ReactNode
  onClick?: () => void
  variant?: 'primary' | 'danger' | 'ghost'
  size?: 'sm'
  disabled?: boolean
  title?: string
}) {
  const cls = ['btn', props.variant, props.size].filter(Boolean).join(' ')
  return (
    <button className={cls} onClick={props.onClick} disabled={props.disabled} title={props.title}>
      {props.children}
    </button>
  )
}

export function Switch({ on, onChange }: { on: boolean; onChange: (v: boolean) => void }) {
  return (
    <button
      className={on ? 'switch on' : 'switch'}
      onClick={() => onChange(!on)}
      aria-pressed={on}
    />
  )
}

export function Modal(props: {
  title: string
  children: ReactNode
  footer?: ReactNode
  onClose: () => void
  width?: number
}) {
  return (
    <div className="mask" onMouseDown={(e) => e.target === e.currentTarget && props.onClose()}>
      <div className="modal" style={props.width ? { width: props.width } : undefined}>
        <div className="modal-head">
          <h3>{props.title}</h3>
          <div className="spacer" />
          <button className="btn ghost sm" onClick={props.onClose}>
            ✕
          </button>
        </div>
        <div className="modal-body">{props.children}</div>
        {props.footer && <div className="modal-foot">{props.footer}</div>}
      </div>
    </div>
  )
}

export function Empty({ icon, title, hint }: { icon: string; title: string; hint?: string }) {
  return (
    <div className="empty">
      <div className="big">{icon}</div>
      <div>{title}</div>
      {hint && (
        <div className="small" style={{ marginTop: 4 }}>
          {hint}
        </div>
      )}
    </div>
  )
}

export function AgentChip({
  agent,
  info,
  onClick,
}: {
  agent: { id: string; name: string; short: string; color: string }
  info?: { deployed: boolean; status: string }
  onClick?: () => void
}) {
  const deployed = info?.deployed ?? false
  const modified = info?.status === 'modified'
  const title = deployed
    ? `${agent.name} · ${modified ? '内容有差异，可重新同步' : '已同步'}`
    : `${agent.name} · 未部署`
  return (
    <span
      className={deployed ? 'agent-chip on' : 'agent-chip'}
      title={title}
      onClick={onClick}
      style={onClick ? { cursor: 'pointer' } : undefined}
    >
      <span className="swatch" style={{ background: deployed ? agent.color : 'var(--border-strong)' }} />
      {agent.short}
      {deployed && <span className="st">{modified ? '≠' : '✓'}</span>}
    </span>
  )
}

export function Spinner({ text }: { text?: string }) {
  return (
    <span className="row small muted" style={{ gap: 8 }}>
      <span className="dot" style={{ background: 'var(--primary)', animation: 'none' }} />
      {text ?? '处理中…'}
    </span>
  )
}
