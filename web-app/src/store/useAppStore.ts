import { create } from 'zustand'
import axios from 'axios'
import type { Run, Benchmark, Language, Framework, Environment, Test } from '../types'

const API_BASE = '/api'

export type AppState = {
  runs: Run[]
  runsLoading: boolean
  languages: Language[]
  frameworks: Framework[]
  languagesLoading: boolean
  fetchLanguages: () => Promise<void>
  fetchRuns: () => Promise<void>
  selectedRunId: number | null
  setSelectedRunId: (id: number | null) => void
  benchmarks: Benchmark[]
  benchmarksLoading: boolean
  fetchBenchmarks: (runId: number | null) => Promise<void>
  environments: Environment[]
  environmentsLoading: boolean
  fetchEnvironments: () => Promise<void>
  selectedEnvironment: string | null
  setSelectedEnvironment: (env: string | null) => void
  tests: Test[]
  testsLoading: boolean
  fetchTests: () => Promise<void>
  selectedTest: string | null
  setSelectedTest: (test: string | null) => void
  theme: 'light' | 'dark'
  toggleTheme: () => void
}

export const useAppStore = create<AppState>((set, get) => ({
  runs: [],
  runsLoading: false,
  languages: [],
  frameworks: [],
  languagesLoading: false,
  environments: [],
  environmentsLoading: false,
  selectedEnvironment: null,
  tests: [],
  testsLoading: false,
  selectedTest: null,
  fetchLanguages: async (): Promise<void> => {
    set({ languagesLoading: true })
    try {
      const langsRes = await axios.get<Language[]>(`${API_BASE}/languages`)
      const fwRes = await axios.get<Framework[]>(`${API_BASE}/frameworks`)
      set({ languages: langsRes.data || [], frameworks: fwRes.data || [], languagesLoading: false })
    } catch {
      set({ languages: [], frameworks: [], languagesLoading: false })
    }
  },
  fetchRuns: async (): Promise<void> => {
    set({ runsLoading: true })
    try {
      const res = await axios.get<Run[]>(`${API_BASE}/runs`)
      const runs = res.data || []
      set({ runs, runsLoading: false })

      // auto-select latest if none
      const currentSelected = get().selectedRunId
      if (!currentSelected && runs.length > 0) {
        const latest = runs.reduce((latest, current) =>
          new Date(current.createdAt) > new Date(latest.createdAt) ? current : latest
        ).id
        get().setSelectedRunId(latest)
      }
    } catch {
      set({ runsLoading: false })
    }
  },
  selectedRunId: null,
  setSelectedRunId: (id: number | null) => {
    set({ selectedRunId: id })
    // fetch benchmarks for this run
    get().fetchBenchmarks(id)
  },
  benchmarks: [],
  benchmarksLoading: false,
  fetchBenchmarks: async (runId: number | null): Promise<void> => {
    if (!runId) {
      set({ benchmarks: [], benchmarksLoading: false })
      return
    }
    const selectedEnv = get().selectedEnvironment
    const selectedTest = get().selectedTest
    if (!selectedEnv || !selectedTest) {
      // filters not loaded yet, skip
      set({ benchmarks: [], benchmarksLoading: false })
      return
    }
    set({ benchmarksLoading: true })
    try {
      const url = `${API_BASE}/runs/${runId}/environments/${selectedEnv}/tests/${selectedTest}`
      const res = await axios.get(url)
      const benchmarks = res.data.map((item: any) => ({
        language: item.language,
        framework: item.framework,
        version: item.version,
        timestamp: item.timestamp,
        rps: item.rps,
        tps: item.tps,
        latencyAvg: item.latencyAvg,
        latencyMax: item.latencyMax,
        latency50: item.latency50,
        latency75: item.latency75,
        latency90: item.latency90,
        latency99: item.latency99,
        errors: item.errors,
        memoryUsage: item.memoryUsage,
        tags: item.tags || {},
      }))
      set({ benchmarks, benchmarksLoading: false })
    } catch {
      set({ benchmarks: [], benchmarksLoading: false })
    }
  },
  fetchEnvironments: async (): Promise<void> => {
    set({ environmentsLoading: true })
    try {
      const res = await axios.get<Environment[]>(`${API_BASE}/environments`)
      const environments = res.data
      set({ environments, environmentsLoading: false })
      // auto-select first if none
      const currentSelected = get().selectedEnvironment
      if (!currentSelected && environments.length > 0) {
        get().setSelectedEnvironment(environments[0].name)
      }
    } catch {
      set({ environments: [], environmentsLoading: false })
    }
  },
  fetchTests: async (): Promise<void> => {
    set({ testsLoading: true })
    try {
      const res = await axios.get<Test[]>(`${API_BASE}/tests`)
      set({ tests: res.data || [], testsLoading: false })
      // auto-select first if none
      const currentSelected = get().selectedTest
      if (!currentSelected && res.data && res.data.length > 0) {
        get().setSelectedTest(res.data[0].id)
      }
    } catch {
      set({ tests: [], testsLoading: false })
    }
  },
  setSelectedEnvironment: (env: string | null) => {
    set({ selectedEnvironment: env })
    // refetch benchmarks if run selected
    const runId = get().selectedRunId
    if (runId) {
      get().fetchBenchmarks(runId)
    }
  },
  setSelectedTest: (test: string | null) => {
    set({ selectedTest: test })
    // refetch benchmarks if run selected
    const runId = get().selectedRunId
    if (runId) {
      get().fetchBenchmarks(runId)
    }
  },
  theme: (localStorage.getItem('theme') as 'light' | 'dark' | null) || 'dark',
  toggleTheme: (): void => {
    const current = get().theme
    const next = current === 'light' ? 'dark' : 'light'
    set({ theme: next })
    localStorage.setItem('theme', next)
    document.documentElement.classList.toggle('dark', next === 'dark')
  },
}))

// initialize theme on load
try {
  const initialTheme = (localStorage.getItem('theme') as 'light' | 'dark' | null) || 'dark'
  if (typeof document !== 'undefined') {
    document.documentElement.classList.toggle('dark', initialTheme === 'dark')
  }
} catch {
  // ignore
}

// Use named export only to satisfy project lint rules (no default exports)
