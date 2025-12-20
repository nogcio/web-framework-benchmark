export interface Run {
  id: number
  started_at: string
}

export interface Benchmark {
  id: number
  run_id: number
  language_id: string
  framework_id: string
  version: string
  timestamp: string
  requests_per_second: number
  latency_avg_ms: number
  latency_max_ms: number
  latency_95th_ms: number
  errors: number
  memory_usage: number
  tags: Record<string, string>
}

export interface Language {
  id: string
  name: string
  link: string
}

export interface Framework {
  id: string
  name: string
  language_id: string
  repo_link: string
}
