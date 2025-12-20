import { useMemo, useState, useEffect } from 'react'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow, TableCaption } from './ui/table'
import { Skeleton } from './ui/skeleton'
import { Empty, EmptyTitle, EmptyDescription, EmptyMedia } from './ui/empty'
import { HoverCard, HoverCardContent, HoverCardTrigger } from './ui/hover-card'
import { Button } from './ui/button'
import { DropdownMenu, DropdownMenuContent, DropdownMenuTrigger, DropdownMenuCheckboxItem } from './ui/dropdown-menu'
import { BarChart3, Settings } from 'lucide-react'
import type { Benchmark } from '../types'
import { useAppStore, type AppState } from '../store/useAppStore'

interface Props {
  benchmarks: Benchmark[]
}

export default function BenchmarksTable({ benchmarks }: Props) {
  const languages = useAppStore((s: AppState) => s.languages)
  const frameworks = useAppStore((s: AppState) => s.frameworks)
  const benchmarksLoading = useAppStore((s: AppState) => s.benchmarksLoading)
  const [localLoading, setLocalLoading] = useState(false)
  const [visibleColumns, setVisibleColumns] = useState({
    framework: true,
    rps: true,
    memory: true,
    tps: true,
    errors: true,
    tags: true,
  })

  const toggleColumn = (column: keyof typeof visibleColumns) => {
    setVisibleColumns(prev => ({ ...prev, [column]: !prev[column] }))
  }

  useEffect(() => {
    if (benchmarksLoading) {
      const timer = setTimeout(() => setLocalLoading(true), 200)
      return () => clearTimeout(timer)
    } else {
      setLocalLoading(false)
    }
  }, [benchmarksLoading])
  const sorted = useMemo(() => {
    return [...benchmarks].sort((a, b) => b.rps - a.rps)
  }, [benchmarks])

  const maxRps = useMemo(() => {
    if (!benchmarks || benchmarks.length === 0) return 0
    return Math.max(...benchmarks.map((b) => b.rps))
  }, [benchmarks])

  function getColorForLanguage(lang: string) {
    let hash = 0
    for (let i = 0; i < lang.length; i++) {
      hash = (hash << 5) - hash + lang.charCodeAt(i)
      hash |= 0
    }
    const hue = Math.abs(hash) % 360
    return `hsl(${hue}, 65%, 50%)`
  }

  function TagsInline({ tags }: { tags: Record<string, string> | undefined }) {
    const entries = Object.entries(tags || {})
    const limit = 3
    const [open, setOpen] = useState(false)

    if (entries.length === 0) {
      return <span className="text-sm text-muted-foreground">—</span>
    }

    const visible = open ? entries : entries.slice(0, limit)
    const hiddenCount = Math.max(0, entries.length - limit)

    return (
      <div className="flex items-center gap-2">
        {visible.map(([k, v]) => (
          <span
            key={`${k}-${v}`}
            className="inline-flex items-center gap-1 rounded-full px-2 py-0.5 bg-muted/30 text-sm text-muted-foreground border border-muted/30"
          >
            <span className="font-medium text-xs">{k}</span>
            {v ? <span className="text-xs">{v}</span> : null}
          </span>
        ))}

        {hiddenCount > 0 && (
          <button
            type="button"
            className="text-sm text-primary ml-1 px-2 py-0.5 rounded hover:bg-muted/20"
            onClick={() => setOpen(!open)}
          >
            {open ? 'show less' : `+${hiddenCount}`}
          </button>
        )}
      </div>
    )
  }

  if (localLoading) {
    return (
      <Table className="w-full text-xs">
        <TableHeader>
          <TableRow>
            <TableHead className="w-1/6 pl-4">Framework</TableHead>
            <TableHead className="w-1/6">Requests/sec</TableHead>
            <TableHead className="w-1/6">Memory</TableHead>
            <TableHead className="w-1/6">TPS</TableHead>
            <TableHead className="w-1/6 pr-4">Tags</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {Array.from({ length: 6 }).map((_, i) => (
            <TableRow key={i}>
              <TableCell className="w-1/6 pl-4">
                <div className="flex items-center">
                  <Skeleton className="w-3 h-3 rounded-sm mr-2" />
                  <Skeleton className="h-4 w-20" />
                </div>
              </TableCell>
              <TableCell className="w-1/6 text-right">
                <Skeleton className="h-4 w-16 ml-auto" />
              </TableCell>
              <TableCell className="w-1/2">
                <div className="flex items-center gap-3">
                  <Skeleton className="flex-1 h-2 rounded" />
                  <Skeleton className="w-12 h-3" />
                </div>
              </TableCell>
              <TableCell className="w-1/4">
                <div className="flex items-center gap-3">
                  <Skeleton className="flex-1 h-2 rounded" />
                  <Skeleton className="w-12 h-3" />
                </div>
              </TableCell>
              <TableCell className="w-1/6">
                <div className="flex items-center gap-3">
                  <Skeleton className="flex-1 h-2 rounded" />
                  <Skeleton className="w-12 h-3" />
                </div>
              </TableCell>
              <TableCell className="w-1/6 pr-4">
                <div className="flex gap-2">
                  <Skeleton className="h-5 w-12 rounded-full" />
                  <Skeleton className="h-5 w-10 rounded-full" />
                </div>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
        <TableCaption>Loading...</TableCaption>
      </Table>
    )
  }

  if (benchmarks.length === 0) {
    return (
      <Empty>
        <EmptyMedia>
          <BarChart3 className="size-12" />
        </EmptyMedia>
        <EmptyTitle>No benchmarks available</EmptyTitle>
        <EmptyDescription>
          No benchmark data found. Try selecting a different run or check your filters.
        </EmptyDescription>
      </Empty>
    )
  }

  return (
    <div>
      <div className="flex justify-end mb-4">
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button variant="outline" size="sm">
              <Settings className="h-4 w-4" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            <DropdownMenuCheckboxItem
              checked={visibleColumns.rps}
              onCheckedChange={() => toggleColumn('rps')}
            >
              Rps
            </DropdownMenuCheckboxItem>
            <DropdownMenuCheckboxItem
              checked={visibleColumns.memory}
              onCheckedChange={() => toggleColumn('memory')}
            >
              Memory
            </DropdownMenuCheckboxItem>
            <DropdownMenuCheckboxItem
              checked={visibleColumns.tps}
              onCheckedChange={() => toggleColumn('tps')}
            >
              TPS
            </DropdownMenuCheckboxItem>
            <DropdownMenuCheckboxItem
              checked={visibleColumns.errors}
              onCheckedChange={() => toggleColumn('errors')}
            >
              Errors
            </DropdownMenuCheckboxItem>
            <DropdownMenuCheckboxItem
              checked={visibleColumns.tags}
              onCheckedChange={() => toggleColumn('tags')}
            >
              Tags
            </DropdownMenuCheckboxItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
      <Table className="w-full text-xs">
        <TableHeader>
          <TableRow>
            {visibleColumns.framework && <TableHead className="w-1/6 pl-4">Framework</TableHead>}
            {visibleColumns.rps && <TableHead className="w-1/6">Requests/sec</TableHead>}
            {visibleColumns.memory && <TableHead className="w-1/6">Memory</TableHead>}
            {visibleColumns.tps && <TableHead className="w-1/6">TPS</TableHead>}
            {visibleColumns.errors && <TableHead className="w-1/12 text-right">Errors</TableHead>}
            {visibleColumns.tags && <TableHead className="w-1/12 pr-4">Tags</TableHead>}
          </TableRow>
        </TableHeader>

        <TableBody>
          {sorted.map((benchmark) => {
            const langColor = getColorForLanguage(benchmark.language || 'unknown')
            // Resolve language/framework URLs from the global store lists
            const language = languages.find((l) => l.name === benchmark.language)
            const framework = frameworks.find((f) => f.name === benchmark.framework)
            const languageHref = language?.url
            const frameworkHref = framework?.url

            return (
              <HoverCard>
                <HoverCardTrigger asChild>
                  <TableRow key={benchmark.language + '-' + benchmark.framework} className="hover:cursor-pointer">
                    {visibleColumns.framework && <TableCell className="w-1/6 pl-4">
                      <div className="flex items-center">
                        <span
                          className="inline-block w-3 h-3 rounded-sm mr-2"
                          style={{ backgroundColor: langColor }}
                        />
                        <div className="font-medium">
                          {languageHref ? (
                            <a href={languageHref} target="_blank" rel="noopener noreferrer" className="text-primary hover:underline">
                              {benchmark.language}
                            </a>
                          ) : (
                            <span>{benchmark.language}</span>
                          )}
                          <span className="mx-1 text-muted-foreground">/</span>
                          {frameworkHref ? (
                            <a href={frameworkHref} target="_blank" rel="noopener noreferrer" className="text-primary hover:underline">
                              {benchmark.framework}
                            </a>
                          ) : (
                            <span>{benchmark.framework}</span>
                          )}
                          <span className="text-[10px] text-muted-foreground lowercase ml-2 leading-none">v{benchmark.version}</span>
                        </div>
                      </div>
                    </TableCell>}

                    {visibleColumns.rps && <TableCell className="w-1/4">
                      <div className="flex items-center gap-3">
                        <div className="text-xs ml-auto font-mono">{(benchmark.rps || 0).toLocaleString()}</div>
                        <div className="flex-1">
                          <div className="w-full h-2 bg-gray-200 dark:bg-gray-700 rounded overflow-hidden">
                            <div
                              className="h-2"
                              style={{
                                width: `${(() => {
                                  const rpsPercent = maxRps > 0 ? (benchmark.rps / maxRps) * 100 : 0
                                  return rpsPercent
                                })()}%`,
                                backgroundColor: langColor,
                              }}
                            />
                          </div>
                        </div>
                        <div className="w-12 text-left text-[10px] font-mono text-muted-foreground leading-none">
                          {(() => {
                            const rpsPercent = maxRps > 0 ? (benchmark.rps / maxRps) * 100 : 0
                            return `${Math.round(rpsPercent)}%`
                          })()}
                        </div>
                      </div>
                    </TableCell>}

                    {visibleColumns.memory && <TableCell className="w-1/4">
                      <div className="flex items-center gap-3">
                        <span className="text-xs font-mono text-muted-foreground mr-2">
                          {(benchmark.memoryUsage / (1024 * 1024)).toFixed(1)}MB
                        </span>
                        <div className="flex-1">
                          <div className="w-full h-2 bg-gray-200 dark:bg-gray-700 rounded overflow-hidden">
                            <div
                              className="h-2"
                              style={{
                                width: `${(() => {
                                  const minMem = Math.min(...benchmarks.map((b) => b.memoryUsage))
                                  const maxMem = Math.max(...benchmarks.map((b) => b.memoryUsage))
                                  const memoryPercent = maxMem > minMem ? ((maxMem - benchmark.memoryUsage) / (maxMem - minMem)) * 100 : 100
                                  return memoryPercent
                                })()}%`,
                                backgroundColor: langColor,
                              }}
                            />
                          </div>
                        </div>

                        <div className="w-12 text-left text-[10px] font-mono text-muted-foreground leading-none">
                          {(() => {
                            const minMem = Math.min(...benchmarks.map((b) => b.memoryUsage))
                            const maxMem = Math.max(...benchmarks.map((b) => b.memoryUsage))
                            const memoryPercent = maxMem > minMem ? ((maxMem - benchmark.memoryUsage) / (maxMem - minMem)) * 100 : 100
                            return `${Math.round(memoryPercent)}%`
                          })()}
                        </div>
                      </div>
                    </TableCell>}

                    {visibleColumns.tps && <TableCell className="w-1/6">
                      <div className="flex items-center gap-3">
                        <span className="text-xs font-mono text-muted-foreground mr-2">
                          {(benchmark.tps / 1000).toFixed(1)}K/s
                        </span>
                        <div className="flex-1">
                          <div className="w-full h-2 bg-gray-200 dark:bg-gray-700 rounded overflow-hidden">
                            <div
                              className="h-2"
                              style={{
                                width: `${(() => {
                                  const maxTps = Math.max(...benchmarks.map((b) => b.tps))
                                  const tpsPercent = maxTps > 0 ? (benchmark.tps / maxTps) * 100 : 0
                                  return tpsPercent
                                })()}%`,
                                backgroundColor: langColor,
                              }}
                            />
                          </div>
                        </div>

                        <div className="w-12 text-left text-[10px] font-mono text-muted-foreground leading-none">
                          {(() => {
                            const maxTps = Math.max(...benchmarks.map((b) => b.tps))
                            const tpsPercent = maxTps > 0 ? (benchmark.tps / maxTps) * 100 : 0
                            return `${Math.round(tpsPercent)}%`
                          })()}
                        </div>
                      </div>
                    </TableCell>}

                    {visibleColumns.errors && <TableCell className="w-1/12 text-right">
                      <span className={benchmark.errors === 0 ? 'text-muted-foreground' : 'text-red-500'}>
                        {benchmark.errors}
                      </span>
                    </TableCell>}

                    {visibleColumns.tags && <TableCell className="w-1/6 pr-4">
                      <TagsInline tags={benchmark.tags} />
                    </TableCell>}
                  </TableRow>
                </HoverCardTrigger>
                <HoverCardContent className="w-80">
                  <div className="space-y-2">
                    <div className="font-semibold flex items-center justify-between">
                      <div className="flex items-center">
                        <span
                          className="inline-block w-3 h-3 rounded-sm mr-2"
                          style={{ backgroundColor: langColor }}
                        />
                        {benchmark.language} / {benchmark.framework}
                      </div>
                      <div className="text-sm">v{benchmark.version}</div>
                    </div>

                    <div className="grid grid-cols-2 gap-2 text-sm">
                      <div><strong>RPS:</strong> {benchmark.rps?.toLocaleString()}</div>
                      <div><strong>Memory:</strong> {(benchmark.memoryUsage / (1024 * 1024)).toFixed(2)} MB</div>
                      <div><strong>TPS:</strong> {(benchmark.tps / (1024 * 1024)).toFixed(2)} MB/s</div>
                      <div><strong>Latency Avg:</strong> {(benchmark.latencyAvg / 1e6).toFixed(2)} ms</div>
                      <div><strong>Errors:</strong> {benchmark.errors}</div>
                    </div>
                    {benchmark.tags && Object.keys(benchmark.tags).length > 0 && (
                      <div className="text-sm">
                        <div className="flex flex-wrap gap-1 mt-1">
                          {Object.entries(benchmark.tags).map(([k, v]) => (
                            <span key={`${k}-${v}`} className="inline-flex items-center gap-1 rounded-full px-2 py-0.5 bg-muted/30 text-xs text-muted-foreground border border-muted/30">
                              <span className="font-medium">{k}</span>
                              {v ? <span>{v}</span> : null}
                            </span>
                          ))}
                        </div>
                      </div>
                    )}
                  </div>
                </HoverCardContent>
              </HoverCard>
            )
          })}
        </TableBody>

        <TableCaption>{sorted.length} results</TableCaption>
      </Table>
    </div>
  )
}
