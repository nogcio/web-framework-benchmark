import { useState, useEffect } from 'react'
import axios from 'axios'
import ReactMarkdown, { type Components } from 'react-markdown'
import type { Benchmark } from '../types'
import { cn, getDatabaseColor, getSortedTags } from '../lib/utils'
import { Tag } from './Tag'
import { useAppStore } from '../store/useAppStore'
import { BenchmarkCharts } from './BenchmarkCharts'

const markdownComponents: Components = {
  h1: ({ className, ...props }) => (
    <h1 className={cn("mt-2 scroll-m-20 text-lg font-bold tracking-tight first:mt-0 text-foreground", className)} {...props} />
  ),
  h2: ({ className, ...props }) => (
    <h2 className={cn("mt-2 scroll-m-20 text-base font-semibold tracking-tight text-foreground", className)} {...props} />
  ),
  h3: ({ className, ...props }) => (
    <h3 className={cn("mt-2 scroll-m-20 text-sm font-semibold tracking-tight text-foreground", className)} {...props} />
  ),
  p: ({ className, ...props }) => (
    <p className={cn("leading-normal [&:not(:first-child)]:mt-2", className)} {...props} />
  ),
  ul: ({ className, ...props }) => (
    <ul className={cn("my-2 ml-6 list-disc [&>li]:mt-1", className)} {...props} />
  ),
  ol: ({ className, ...props }) => (
    <ol className={cn("my-2 ml-6 list-decimal [&>li]:mt-1", className)} {...props} />
  ),
  li: ({ className, ...props }) => (
    <li className={cn("", className)} {...props} />
  ),
  blockquote: ({ className, ...props }) => (
    <blockquote className={cn("mt-2 border-l-2 pl-6 italic", className)} {...props} />
  ),
  pre: ({ className, ...props }) => (
    <pre className={cn("overflow-x-auto rounded-lg border bg-muted p-2 my-2 [&_code]:bg-transparent [&_code]:p-0 [&_code]:font-normal", className)} {...props} />
  ),
  code: ({ className, ...props }) => (
    <code className={cn("relative rounded bg-muted px-[0.3rem] py-[0.2rem] font-mono text-xs font-semibold text-foreground", className)} {...props} />
  ),
  strong: ({ className, ...props }) => (
    <strong className={cn("font-bold text-foreground", className)} {...props} />
  ),
}

interface BenchmarkHoverDetailsProps {
  benchmark: Benchmark
  langColor: string
}

export function BenchmarkHoverDetails({ benchmark, langColor }: BenchmarkHoverDetailsProps) {
  const [transcript, setTranscript] = useState<string | null>(null)
  const [loading, setLoading] = useState(false)
  const selectedRunId = useAppStore(s => s.selectedRunId)
  const selectedEnvironment = useAppStore(s => s.selectedEnvironment)
  const selectedTest = useAppStore(s => s.selectedTest)
  const benchmarksLoading = useAppStore(s => s.benchmarksLoading)
  const transcripts = useAppStore(s => s.transcripts)
  const addTranscript = useAppStore(s => s.addTranscript)

  useEffect(() => {
    if (benchmark.hasTranscript && selectedRunId && selectedEnvironment && selectedTest && !benchmarksLoading) {
      const lang = navigator.language.split('-')[0]
      const cacheKey = `${selectedRunId}:${selectedEnvironment}:${selectedTest}:${benchmark.name}:${lang}`
      
      if (transcripts[cacheKey]) {
        setTimeout(() => {
          setTranscript(transcripts[cacheKey])
          setLoading(false)
        }, 0)
        return
      }

      setTimeout(() => setLoading(true), 0)
      const controller = new AbortController()
      
      axios.get(`/api/runs/${selectedRunId}/environments/${selectedEnvironment}/tests/${selectedTest}/frameworks/${benchmark.name}/transcript`, {
        params: { lang },
        responseType: 'text',
        signal: controller.signal
      })
      .then(res => {
        setTranscript(res.data)
        addTranscript(cacheKey, res.data)
      })
      .catch((err) => {
        if (!axios.isCancel(err)) {
          setTranscript(null)
        }
      })
      .finally(() => {
        if (!controller.signal.aborted) {
          setLoading(false)
        }
      })

      return () => controller.abort()
    } else {
      setTimeout(() => setTranscript(null), 0)
    }
  }, [benchmark, selectedRunId, selectedEnvironment, selectedTest, benchmarksLoading, transcripts, addTranscript])

  const showTranscript = benchmark.hasTranscript

  return (
    <div className={cn("p-4 group", showTranscript ? "w-[1200px]" : "w-[600px]")}>
      <div className={cn("flex gap-4", showTranscript && "h-[600px]")}>
        <div className={cn("space-y-2 flex flex-col", showTranscript ? "w-1/3 shrink-0 min-w-[350px]" : "w-full")}>
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
          
          <div className="flex-1 min-h-0 overflow-y-auto">
             <BenchmarkCharts benchmarkName={benchmark.name} />
          </div>
        </div>

        {showTranscript && (
          <>
            <div className="w-px bg-border" />
            <div className="flex-1 overflow-y-auto pr-2 scrollbar-thin">
              <div className="text-xs font-semibold mb-2 sticky top-0 bg-popover pb-2 border-b z-10">AI Analysis</div>
              {loading || !transcript ? (
                <div className="space-y-2">
                  <div className="h-4 w-3/4 bg-muted animate-pulse rounded" />
                  <div className="h-4 w-full bg-muted animate-pulse rounded" />
                  <div className="h-4 w-5/6 bg-muted animate-pulse rounded" />
                  <div className="h-4 w-full bg-muted animate-pulse rounded" />
                  <div className="h-4 w-2/3 bg-muted animate-pulse rounded" />
                </div>
              ) : (
                <div className="text-xs text-zinc-400">
                  <ReactMarkdown components={markdownComponents}>{transcript}</ReactMarkdown>
                </div>
              )}
            </div>
          </>
        )}
      </div>
    </div>
  )
}
