import './App.css'
import Header from './components/Header'
import Footer from './components/Footer'
import RunsTabs from './components/RunsTabs'
import EnvironmentDetails from './components/EnvironmentDetails'
import { useEffect } from 'react'
import { useAppStore, type AppState } from './store/useAppStore'
import { useUrlSync } from './hooks/useUrlSync'

let appInitialized = false

function App() {
  useUrlSync()
  const fetchLanguages = useAppStore((s: AppState) => s.fetchLanguages)
  const fetchRuns = useAppStore((s: AppState) => s.fetchRuns)
  const fetchEnvironments = useAppStore((s: AppState) => s.fetchEnvironments)
  const fetchTests = useAppStore((s: AppState) => s.fetchTests)
  const selectedRunId = useAppStore((s: AppState) => s.selectedRunId)
  const selectedEnvironment = useAppStore((s: AppState) => s.selectedEnvironment)
  const selectedTest = useAppStore((s: AppState) => s.selectedTest)
  const fetchBenchmarks = useAppStore((s: AppState) => s.fetchBenchmarks)

  // Fetch benchmarks when filters change
  useEffect(() => {
    if (selectedRunId && selectedEnvironment && selectedTest) {
      const controller = new AbortController()
      fetchBenchmarks(selectedRunId, controller.signal)
      return () => controller.abort()
    }
  }, [selectedRunId, selectedEnvironment, selectedTest, fetchBenchmarks])

  useEffect(() => {
    if (appInitialized) return
    appInitialized = true

    const load = async () => {
      await Promise.all([
        fetchLanguages(),
        fetchRuns(),
        fetchEnvironments(),
        fetchTests()
      ])
    }
    void load()
  }, [fetchLanguages, fetchRuns, fetchEnvironments, fetchTests])

  return (
    <div className="h-screen bg-background text-foreground w-full p-4 flex flex-col overflow-hidden">
      <Header />

      <div className="-mx-4 flex-1 overflow-hidden flex flex-col">
        <RunsTabs />
      </div>
      <EnvironmentDetails />
      <div className="-mx-4 -mb-4">
        <Footer />
      </div>
    </div>
  )
}

export default App
