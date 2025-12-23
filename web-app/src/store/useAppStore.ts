import { create } from 'zustand'
import axios from 'axios'
import type { Run, Benchmark, Language, Framework, Environment, Test, VisibleColumns } from '../types'

const API_BASE = '/api'

type BenchmarkResponse = Benchmark & {
  tags?: Record<string, string>
}

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
  fetchBenchmarks: (runId: number | null, signal?: AbortSignal) => Promise<void>
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
  visibleColumns: VisibleColumns
  setVisibleColumns: (columns: VisibleColumns) => void
  toggleColumn: (column: keyof VisibleColumns) => void
  transcripts: Record<string, string>
  addTranscript: (key: string, content: string) => void
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
  transcripts: {},
  addTranscript: (key, content) => set((state) => ({
    transcripts: { ...state.transcripts, [key]: content }
  })),
  visibleColumns: {
    rank: true,
    framework: true,
    rps: true,
    memory: true,
    memoryBar: false,
    tps: true,
    tpsBar: false,
    errors: true,
    tags: false,
  },
  setVisibleColumns: (columns) => set({ visibleColumns: columns }),
  toggleColumn: (column) => set((state) => ({
    visibleColumns: {
      ...state.visibleColumns,
      [column]: !state.visibleColumns[column]
    }
  })),
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
        // Select the run with the highest ID (assuming higher ID = newer)
        const latest = runs.reduce((max, current) => 
          current.id > max.id ? current : max
        ).id
        get().setSelectedRunId(latest)
      }
    } catch {
      set({ runsLoading: false })
    }
  },
  selectedRunId: null,
  setSelectedRunId: (id: number | null) => {
    const { selectedEnvironment, selectedTest } = get()
    const shouldLoad = id !== null && selectedEnvironment !== null && selectedTest !== null
    set({ selectedRunId: id, benchmarksLoading: shouldLoad })
  },
  benchmarks: [],
  benchmarksLoading: false,
  fetchBenchmarks: async (runId: number | null, signal?: AbortSignal): Promise<void> => {
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
    // Don't set loading true here if it was already set by the filter change
    // But we need to ensure it's true if called directly
    set({ benchmarksLoading: true }) 
    
    try {
      const url = `${API_BASE}/runs/${runId}/environments/${selectedEnv}/tests/${selectedTest}`
      const res = await axios.get<BenchmarkResponse[]>(url, { signal })
      const benchmarks = (res.data || []).map((item) => ({
        ...item,
        tags: item.tags || {},
      }))
      set({ benchmarks, benchmarksLoading: false })
    } catch (err) {
      if (!axios.isCancel(err)) {
        set({ benchmarks: [], benchmarksLoading: false })
      }
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
    const { selectedRunId, selectedTest } = get()
    const shouldLoad = env !== null && selectedRunId !== null && selectedTest !== null
    set({ selectedEnvironment: env, benchmarksLoading: shouldLoad })
  },
  setSelectedTest: (test: string | null) => {
    const { selectedRunId, selectedEnvironment } = get()
    const shouldLoad = test !== null && selectedRunId !== null && selectedEnvironment !== null
    set({ selectedTest: test, benchmarksLoading: shouldLoad })
  },
}))

// initialize theme on load
try {
  if (typeof document !== 'undefined') {
    document.documentElement.classList.add('dark')
  }
} catch {
  // ignore
}

// Use named export only to satisfy project lint rules (no default exports)
