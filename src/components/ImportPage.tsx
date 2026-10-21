import { useState } from 'react'
import { api, pickFolder, ScanResult, StateSnapshot } from '../api'
import { Button, Empty } from './ui'

type Notify = (msg: string, kind?: 'ok' | 'err') => void

export default function ImportPage(props: {
  state: StateSnapshot
  reload: (silent?: boolean) => Promise<void>
  notify: Notify
  setBusy: (v: boolean) => void
}) {
  const { state, reload, notify, setBusy } = props
  const [mode, setMode] = useState<'local' | 'github'>('local')
  const [localPath, setLocalPath] = useState('')
  const [repoUrl, setRepoUrl] = useState('')
  const [scan, setScan] = useState<ScanResult | null>(null)
  const [picked, setPicked] = useState<string[]>([])
  const [scanning, setScanning] = useState(false)

  const browse = async () => {
    const p = await pickFolder(state.home)
    if (p) setLocalPath(p)
  }

  const reset = async () => {
    if (scan?.session) {
      try {
        await api.cancelScan(scan.session)
      } catch {
        /* ignore */
      }
    }
    setScan(null)
    setPicked([])
  }

  const doScan = async () => {
    const input = mode === 'local' ? localPath.trim() : repoUrl.trim()
    if (!input) {
      notify(mode === 'local' ? '请先选择本地目录' : '请输入 GitHub 仓库地址', 'err')
      return
    }
    await reset()
    setScanning(true)
    try {
      const res = await api.scanSource(mode, input)
      setScan(res)
      setPicked(res.candidates.map((c) => c.id))
      notify(`发现 ${res.candidates.length} 个可导入项`)
    } catch (e) {
      notify(String(e), 'err')
    } finally {
      setScanning(false)
    }
  }

  const doImport = async () => {
    if (!scan || picked.length === 0) return
    setBusy(true)
    try {
      const metas = await api.importSkills(scan.session, picked)
      notify(`已导入 ${metas.length} 个 Skill：${metas.map((m) => m.name).join('、')}`)
      setScan(null)
      setPicked([])
      await reload(true)
    } catch (e) {
      notify(String(e), 'err')
    } finally {
      setBusy(false)
    }
  }

  const toggle = (id: string) =>
    setPicked((prev) => (prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id]))

  return (
    <div>
      <div className="card" style={{ padding: 16, marginBottom: 14 }}>
        <div className="tabs" style={{ marginBottom: 14 }}>
          <button className={mode === 'local' ? 'active' : ''} onClick={() => { setMode('local'); reset() }}>
            本地目录
          </button>
          <button className={mode === 'github' ? 'active' : ''} onClick={() => { setMode('github'); reset() }}>
            GitHub 仓库
          </button>
        </div>

        {mode === 'local' ? (
          <div className="row">
            <input
              className="input"
              style={{ flex: 1 }}
              placeholder="例如 C:\Users\me\my-skills 或某个包含多个 skill 的父目录"
              value={localPath}
              onChange={(e) => setLocalPath(e.target.value)}
            />
            <Button onClick={browse}>浏览…</Button>
            <Button variant="primary" onClick={doScan} disabled={scanning}>
              {scanning ? '扫描中…' : '扫描'}
            </Button>
          </div>
        ) : (
          <>
            <div className="row">
              <input
                className="input"
                style={{ flex: 1 }}
                placeholder="https://github.com/owner/repo 或 .../tree/main/skills/foo"
                value={repoUrl}
                onChange={(e) => setRepoUrl(e.target.value)}
              />
              <Button variant="primary" onClick={doScan} disabled={scanning}>
                {scanning ? '拉取中…' : '拉取并扫描'}
              </Button>
            </div>
            <div className="hint" style={{ marginTop: 8 }}>
              支持 <span className="mono">owner/repo</span>、完整 URL，以及带
              <span className="mono"> /tree/分支/子目录 </span>
              的地址。仓库里所有含 SKILL.md 的文件夹都会被列出供你勾选。
            </div>
          </>
        )}

        {mode === 'local' && (
          <div className="hint" style={{ marginTop: 8 }}>
            可以直接选择一个 skill 目录，也可以选择包含多个 skill 的父目录（会自动向下查找
            SKILL.md）。导入后会在本地仓库留一份正本，原目录不会被改动。
          </div>
        )}
      </div>

      {!scan ? (
        <div className="card">
          <Empty icon="⬇️" title="选择来源后点击扫描" hint="扫描结果会在这里列出，勾选后再导入" />
        </div>
      ) : (
        <div>
          <div className="toolbar">
            <strong>扫描结果</strong>
            <span className="chip">{scan.candidates.length} 项</span>
            <span className="small muted mono truncate" style={{ maxWidth: 420 }}>
              {scan.origin}
            </span>
            <div className="spacer" />
            <Button size="sm" onClick={() => setPicked(scan.candidates.map((c) => c.id))}>
              全选
            </Button>
            <Button size="sm" onClick={() => setPicked([])}>
              清空
            </Button>
            <Button size="sm" onClick={reset}>
              重新扫描
            </Button>
            <Button size="sm" variant="primary" onClick={doImport} disabled={picked.length === 0}>
              导入选中的 {picked.length} 项
            </Button>
          </div>

          {scan.candidates.map((c) => (
            <div
              key={c.id}
              className={picked.includes(c.id) ? 'list-item checked' : 'list-item'}
              onClick={() => toggle(c.id)}
            >
              <input
                type="checkbox"
                checked={picked.includes(c.id)}
                onChange={() => toggle(c.id)}
                style={{ marginTop: 3 }}
              />
              <div style={{ minWidth: 0, flex: 1 }}>
                <div className="row">
                  <strong>{c.name}</strong>
                  {!c.hasSkillFile && <span className="chip warn">无 SKILL.md</span>}
                  {c.rel !== '.' && <span className="chip mono">{c.rel}</span>}
                </div>
                {c.description && (
                  <div className="small muted clamp-2">{c.description}</div>
                )}
                <div className="small muted" style={{ marginTop: 2 }}>
                  {c.fileCount} 个文件
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
