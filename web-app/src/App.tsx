import './App.css'
import Header from './components/Header'
import RunsTabs from './components/RunsTabs'
import { useEffect } from 'react'
import { useAppStore, type AppState } from './store/useAppStore'

function App() {
  const fetchLanguages = useAppStore((s: AppState) => s.fetchLanguages)
  const fetchRuns = useAppStore((s: AppState) => s.fetchRuns)
  const languagesLoading = useAppStore((s: AppState) => s.languagesLoading)
  const runsLoading = useAppStore((s: AppState) => s.runsLoading)

  useEffect(() => {
    const load = async () => {
      await fetchLanguages()
      await fetchRuns()
    }
    void load()
  }, [fetchLanguages, fetchRuns])

  const initialLoading = languagesLoading || runsLoading

  if (initialLoading) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-background text-foreground">
        <div className="flex flex-col items-center gap-4">
          <div className="h-10 w-10 rounded-full border-4 border-muted/30 border-t-primary animate-spin" />
          <div className="text-sm text-muted-foreground">Loading application data…</div>
        </div>
      </div>
    )
  }

  return (
      <div className="min-h-screen bg-background text-foreground w-full p-4">
        <Header />
        <div className="-mx-4">
          <RunsTabs />
        </div>
      </div>
  )
}

export default App
