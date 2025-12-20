import { create } from 'zustand'
import axios from 'axios'
import type { Run, Benchmark, Language, Framework } from '../types'

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
  theme: 'light' | 'dark'
  toggleTheme: () => void
}

export const useAppStore = create<AppState>((set, get) => ({
  runs: [],
  runsLoading: false,
  languages: [],
  frameworks: [],
  languagesLoading: false,
  fetchLanguages: async (): Promise<void> => {
    set({ languagesLoading: true })
    try {
      const [langsRes, fwRes] = await Promise.all([
        axios.get<Language[]>(`${API_BASE}/languages`),
        axios.get<Framework[]>(`${API_BASE}/frameworks`),
      ])
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
          new Date(current.started_at) > new Date(latest.started_at) ? current : latest
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
    set({ benchmarksLoading: true })
    try {
      const res = await axios.get<Benchmark[]>(`${API_BASE}/runs/${runId}/benchmarks`)
      set({ benchmarks: res.data || [], benchmarksLoading: false })
    } catch {
      set({ benchmarks: [], benchmarksLoading: false })
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
