import './App.css'
import Header from './components/Header'
import Footer from './components/Footer'
import RunsTabs from './components/RunsTabs'
import EnvironmentDetails from './components/EnvironmentDetails'
import { useEffect } from 'react'
import { useAppStore, type AppState } from './store/useAppStore'
import { useUrlSync } from './hooks/useUrlSync'

function App() {
  useUrlSync()
  const fetchLanguages = useAppStore((s: AppState) => s.fetchLanguages)
  const fetchRuns = useAppStore((s: AppState) => s.fetchRuns)
  const fetchEnvironments = useAppStore((s: AppState) => s.fetchEnvironments)
  const fetchTests = useAppStore((s: AppState) => s.fetchTests)

  useEffect(() => {
    const load = async () => {
      await Promise.all([
        fetchLanguages(),
        fetchRuns(),
        fetchEnvironments(),
        fetchTests()
      ])
      // after all loaded, fetch benchmarks for selected run
      const selectedRunId = useAppStore.getState().selectedRunId
      if (selectedRunId) {
        useAppStore.getState().fetchBenchmarks(selectedRunId)
      }
    }
    void load()
  }, [fetchLanguages, fetchRuns, fetchEnvironments, fetchTests])

  return (
    <div className="min-h-screen bg-background text-foreground w-full p-4 flex flex-col">
      <Header />

      <div className="-mx-4 flex-1">
        <RunsTabs />
      </div>
      <EnvironmentDetails />
      <div className="-mx-4 -mb-4 mt-4">
        <Footer />
      </div>
    </div>
  )
}

export default App
