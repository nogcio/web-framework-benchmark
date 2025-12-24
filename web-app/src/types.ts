export interface Run {
  id: number
  createdAt: string
}

export interface Environment {
  name: string
  displayName: string
  icon: string
  spec?: string
}

export interface Benchmark {
  name: string
  language: string
  languageVersion: string
  framework: string
  frameworkVersion: string
  database: string | null
  path?: string
  rps: number
  tps: number
  latencyAvg: number
  latencyMax: number
  latency50: number
  latency75: number
  latency90: number
  latency99: number
  errors: number
  memoryUsage: number
  tags: Record<string, string>
  hasTranscript?: boolean
}

export interface Language {
  name: string
  url: string
  color: string
}

export interface Framework {
  language: string
  name: string
  url: string
}

export interface BenchmarkDefinition {
  name: string
  language: string
  languageVersion: string
  framework: string
  frameworkVersion: string
  tests: string[]
  tags: Record<string, string>
  path: string
  database: string
  arguments?: string[]
  env?: Record<string, string>
}

export interface Test {
  id: string | null
  name: string
  icon: string
  children?: Test[]
}

export interface VisibleColumns {
  rank: boolean
  framework: boolean
  rps: boolean
  memory: boolean
  memoryBar: boolean
  tps: boolean
  tpsBar: boolean
  errors: boolean
  tags: boolean
}
