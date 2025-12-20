import React from 'react'
import { Tabs, TabsContent, TabsList, TabsTrigger } from './ui/tabs'
import BenchmarksTable from './BenchmarksTable'
import { useAppStore, type AppState } from '../store/useAppStore'

export default function RunsTabs() {
  const runs = useAppStore((s: AppState) => s.runs)
  const selectedRunId = useAppStore((s: AppState) => s.selectedRunId)
  const setSelectedRunId = useAppStore((s: AppState) => s.setSelectedRunId)
  const benchmarks = useAppStore((s: AppState) => s.benchmarks)
  const benchmarksLoading = useAppStore((s: AppState) => s.benchmarksLoading)

  return (
    <Tabs value={selectedRunId?.toString()} onValueChange={(value) => setSelectedRunId(Number(value))}>
      <TabsList className="w-full ">
        {runs.map((run) => (
          <TabsTrigger key={run.id} value={run.id.toString()} className="flex flex-col items-center">
            <div className="font-medium">Run {run.id}</div>
            <div className="text-xs text-muted-foreground mt-1">
              {new Date(run.started_at).toLocaleDateString()}
            </div>
          </TabsTrigger>
        ))}
      </TabsList>

      {runs.map((run) => (
        <TabsContent key={run.id} value={run.id.toString()}>
            <div className="flex flex-col">
              {benchmarksLoading ? (
                <div>Loading benchmarks...</div>
              ) : benchmarks && benchmarks.length > 0 ? (
                <BenchmarksTable benchmarks={benchmarks} />
              ) : (
                <div>No benchmarks found for this run.</div>
              )}
            </div>
        </TabsContent>
      ))}
    </Tabs>
  )
}
