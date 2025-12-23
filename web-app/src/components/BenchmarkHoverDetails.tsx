import { HoverCardContent } from './ui/hover-card'
import type { Benchmark } from '../types'
import { cn, getDatabaseColor, getSortedTags } from '../lib/utils'
import { Tag } from './Tag'

interface BenchmarkHoverDetailsProps {
  benchmark: Benchmark
  langColor: string
}

export function BenchmarkHoverDetails({ benchmark, langColor }: BenchmarkHoverDetailsProps) {
  return (
    <HoverCardContent className="w-80">
      <div className="space-y-2">
        <div className="font-semibold flex flex-col gap-1">
          <div className="flex items-center justify-between gap-2">
            <div className="flex items-center min-w-0">
              <span
                className="inline-block w-3 h-3 rounded-sm mr-2 shrink-0"
                style={{ backgroundColor: langColor }}
              />
              <span className="truncate">
                {benchmark.language} / {benchmark.framework}
              </span>
            </div>
            <div className="flex items-center gap-2 shrink-0">
              <span className="text-xs text-muted-foreground font-mono">{benchmark.name}</span>
              {benchmark.database && (
                <span className={cn("inline-flex items-center rounded-md border px-1.5 py-0.5 text-[10px] font-semibold transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 border-transparent", getDatabaseColor(benchmark.database))}>
                  {benchmark.database}
                </span>
              )}
            </div>
          </div>
          <div className="text-xs text-muted-foreground flex gap-2">
            <span>{benchmark.language} v{benchmark.languageVersion}</span>
            <span>•</span>
            <span>{benchmark.framework} v{benchmark.frameworkVersion}</span>
          </div>
        </div>

        <div className="grid grid-cols-2 gap-2 text-sm">
          <div><strong>RPS:</strong> {benchmark.rps?.toLocaleString()}</div>
          <div><strong>Memory:</strong> {(benchmark.memoryUsage / (1024 * 1024)).toFixed(2)}MB</div>
          <div><strong>TPS:</strong> {(benchmark.tps / (1024 * 1024)).toFixed(2)}MB/s</div>
          <div><strong>Latency:</strong> {(benchmark.latencyAvg / 1e6).toFixed(2)}ms</div>
          <div><strong>Errors:</strong> {benchmark.errors}</div>
        </div>
        {benchmark.tags && Object.keys(benchmark.tags).length > 0 && (
          <div className="text-sm">
            <div className="flex flex-wrap gap-1 mt-1">
              {getSortedTags(benchmark.tags).map(([k, v]) => (
                <Tag key={`${k}-${v}`} k={k} v={v} />
              ))}
            </div>
          </div>
        )}
      </div>
    </HoverCardContent>
  )
}
