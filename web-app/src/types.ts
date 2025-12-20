export interface Run {
  id: number
  createdAt: string
}

export interface Environment {
  name: string
  displayName: string
  icon: string
}

export interface Benchmark {
  language: string
  framework: string
  version: string
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
}

export interface Language {
  name: string
  url: string
}

export interface Framework {
  language: string
  name: string
  url: string
  tags: Record<string, string>
}

export interface Environment {
  id: string
  name: string
}

export interface Test {
  id: string
  name: string
}
