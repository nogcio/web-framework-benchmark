import { Tabs, TabsContent, TabsList, TabsTrigger } from './ui/tabs'
import BenchmarksTable from './BenchmarksTable'
import { useAppStore, type AppState } from '../store/useAppStore'

export default function RunsTabs() {
  const runs = useAppStore((s: AppState) => s.runs)
  const selectedRunId = useAppStore((s: AppState) => s.selectedRunId)
  const setSelectedRunId = useAppStore((s: AppState) => s.setSelectedRunId)
  const benchmarks = useAppStore((s: AppState) => s.benchmarks)

  return (
    <Tabs 
      value={selectedRunId?.toString()} 
      onValueChange={(value) => setSelectedRunId(Number(value))}
      orientation="vertical"
      className="flex flex-row w-full items-start gap-0"
    >
      <TabsList className="flex flex-col h-auto w-auto space-y-0 bg-transparent p-0 border-0 ml-2 mr-0 z-10 mt-7">
        {[...runs].sort((a, b) => b.id - a.id).map((run) => (
          <TabsTrigger 
            key={run.id} 
            value={run.id.toString()} 
            className="flex flex-col items-start w-20 rounded-none border border-r-0 border-border/50 bg-muted shadow-md data-[state=active]:bg-background data-[state=active]:shadow-none data-[state=active]:font-bold px-2 py-2 transition-all hover:bg-muted/80 text-foreground dark:text-foreground"
          >
            <div className="text-sm">Run {run.id}</div>
            <div className="text-[10px] text-muted-foreground mt-0.5">
              {new Date(run.createdAt).toLocaleDateString()}
            </div>
          </TabsTrigger>
        ))}
      </TabsList>

      <div className="flex-1 bg-background min-h-[500px] p-0">
        {runs.map((run) => (
          <TabsContent key={run.id} value={run.id.toString()} className="mt-0 m-0 border-0 p-0">
              <div className="flex flex-col">
                <BenchmarksTable benchmarks={benchmarks} />
              </div>
          </TabsContent>
        ))}
      </div>
    </Tabs>
  )
}
