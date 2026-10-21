import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'

export type DeployStatus = 'synced' | 'modified' | 'missing'

export interface DeployInfo {
  deployed: boolean
  status: DeployStatus
  path: string
  updatedAt: string | null
}

export interface SkillSource {
  kind: 'local' | 'github' | string
  origin: string | null
  owner: string | null
  repo: string | null
  reference: string | null
  subpath: string | null
}

export interface SkillMeta {
  id: string
  name: string
  description: string
  version: string | null
  author: string | null
  license: string | null
  tags: string[]
  source: SkillSource
  path: string
  fileCount: number
  sizeBytes: number
  hash: string
  importedAt: string
  updatedAt: string
}

export interface SkillView extends SkillMeta {
  deployments: Record<string, DeployInfo>
}

export interface AgentView {
  id: string
  name: string
  short: string
  kind: 'builtin' | 'custom'
  path: string
  ready: boolean
  installed: boolean
  writable: boolean
  skillCount: number
  managedCount: number
  enabled: boolean
  color: string
  homepage: string | null
}

export interface CustomAgent {
  id: string
  name: string
  path: string
  color: string
}

export interface Config {
  storeDir: string
  disabledAgents: string[]
  agentOverrides: Record<string, string>
  customAgents: CustomAgent[]
  autoSync: boolean
  theme: string
  overwriteOnDeploy: boolean
}

export interface Candidate {
  id: string
  name: string
  description: string
  path: string
  rel: string
  hasSkillFile: boolean
  fileCount: number
}

export interface ScanResult {
  session: string
  kind: string
  origin: string
  candidates: Candidate[]
  message: string | null
}

export interface OpResult {
  ok: boolean
  skillId: string
  agentId: string
  message: string
  path: string | null
}

export interface LogEntry {
  time: string
  level: string
  message: string
}

export interface StateSnapshot {
  skills: SkillView[]
  agents: AgentView[]
  config: Config
  platform: string
  home: string
}

export const api = {
  getState: () => invoke<StateSnapshot>('get_state'),

  scanSource: (kind: 'local' | 'github', input: string) =>
    invoke<ScanResult>('scan_source', { kind, input }),

  importSkills: (session: string, ids: string[]) =>
    invoke<SkillMeta[]>('import_skills', { session, ids }),

  cancelScan: (session: string) => invoke<void>('cancel_scan', { session }),

  deleteSkill: (id: string, undeployAll: boolean) =>
    invoke<OpResult[]>('delete_skill', { id, undeployAll }),

  deploySelection: (skillIds: string[], agentIds: string[], overwrite?: boolean) =>
    invoke<OpResult[]>('deploy_selection', {
      skillIds,
      agentIds,
      overwrite: overwrite ?? null,
    }),

  undeploySelection: (skillIds: string[], agentIds: string[]) =>
    invoke<OpResult[]>('undeploy_selection', { skillIds, agentIds }),

  syncDeployed: (agentIds: string[]) => invoke<OpResult[]>('sync_deployed', { agentIds }),

  updateFromSource: (ids: string[]) => invoke<OpResult[]>('update_from_source', { ids }),

  saveConfig: (config: Config) => invoke<Config>('save_config', { config }),

  addCustomAgent: (name: string, path: string) =>
    invoke<Config>('add_custom_agent', { name, path }),

  removeCustomAgent: (id: string) => invoke<Config>('remove_custom_agent', { id }),

  setAgentPath: (id: string, path: string) => invoke<Config>('set_agent_path', { id, path }),

  toggleAgent: (id: string, enabled: boolean) => invoke<Config>('toggle_agent', { id, enabled }),

  revealPath: (path: string) => invoke<void>('reveal_path', { path }),

  getLogs: () => invoke<LogEntry[]>('get_logs'),
}

/** 选择文件夹（原生对话框） */
export async function pickFolder(start?: string): Promise<string | null> {
  const res = await open({ directory: true, multiple: false, defaultPath: start })
  if (Array.isArray(res)) return res[0] ?? null
  return res ?? null
}

export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`
}

export function statusLabel(s: DeployStatus): string {
  if (s === 'synced') return '已同步'
  if (s === 'modified') return '已改动'
  return '未部署'
}
