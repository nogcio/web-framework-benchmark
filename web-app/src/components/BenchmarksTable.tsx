import { useMemo, useState, useEffect } from 'react'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow, TableCaption } from './ui/table'
import { Skeleton } from './ui/skeleton'
import { Empty, EmptyTitle, EmptyDescription, EmptyMedia } from './ui/empty'
import { HoverCard, HoverCardTrigger } from './ui/hover-card'
import { BarChart3 } from 'lucide-react'
import type { Benchmark, VisibleColumns } from '../types'
import { useAppStore, type AppState } from '../store/useAppStore'
import { TagsInline } from './TagsInline'
import { getColorForLanguage, formatNumber, cn, getDatabaseColor } from '../lib/utils'
import { TableSettings } from './TableSettings'
import { BenchmarkHoverDetails } from './BenchmarkHoverDetails'

interface Props {
  benchmarks: Benchmark[]
}

export default function BenchmarksTable({ benchmarks }: Props) {
  const languages = useAppStore((s: AppState) => s.languages)
  const frameworks = useAppStore((s: AppState) => s.frameworks)
  const benchmarksLoading = useAppStore((s: AppState) => s.benchmarksLoading)
  const visibleColumns = useAppStore((s: AppState) => s.visibleColumns)
  const toggleColumn = useAppStore((s: AppState) => s.toggleColumn)
  const [localLoading, setLocalLoading] = useState(false)

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

  const columnOrder: (keyof VisibleColumns)[] = ['rank', 'framework', 'rps', 'memory', 'memoryBar', 'tps', 'tpsBar', 'errors', 'tags']
  const lastVisibleColumn = columnOrder.filter(c => visibleColumns[c]).pop()

  const settingsMenu = (
    <TableSettings visibleColumns={visibleColumns} onToggleColumn={toggleColumn} />
  )

  if (localLoading) {
    return (
      <Table containerClassName="h-full overflow-auto bg-[linear-gradient(90deg,var(--primary)_1px,transparent_1px)]" className="w-full text-xs">
        <TableHeader className="sticky top-0 z-20 bg-background">
          <TableRow>
            {visibleColumns.rank && <TableHead className="w-8 pl-4">#</TableHead>}
            {visibleColumns.framework && <TableHead className="w-[20%] pl-4">Framework</TableHead>}
            {visibleColumns.rps && <TableHead className="w-auto">Requests/sec</TableHead>}
            {visibleColumns.memory && <TableHead className="w-px whitespace-nowrap px-2">Memory</TableHead>}
            {visibleColumns.memoryBar && <TableHead className="w-[15%]"></TableHead>}
            {visibleColumns.tps && <TableHead className="w-px whitespace-nowrap px-2">TPS</TableHead>}
            {visibleColumns.tpsBar && <TableHead className="w-[15%]"></TableHead>}
            {visibleColumns.errors && <TableHead className="w-px text-right whitespace-nowrap">Errors</TableHead>}
            {visibleColumns.tags && <TableHead className="w-24 pr-4">Tags</TableHead>}
            <TableHead className="w-[var(--spacing)] p-0"></TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {Array.from({ length: 6 }).map((_, i) => (
            <TableRow key={i}>
              {visibleColumns.rank && <TableCell className="w-8 pl-4"><Skeleton className="h-4 w-4" /></TableCell>}
              {visibleColumns.framework && <TableCell className="w-[20%] pl-4">
                <div className="flex items-center">
                  <Skeleton className="w-3 h-3 rounded-sm mr-2" />
                  <Skeleton className="h-4 w-20" />
                </div>
              </TableCell>}
              {visibleColumns.rps && <TableCell className="w-auto">
                <div className="flex items-center gap-3">
                  <Skeleton className="flex-1 h-2 rounded" />
                  <Skeleton className="w-12 h-3" />
                </div>
              </TableCell>}
              {visibleColumns.memory && <TableCell className="w-px whitespace-nowrap px-2"><Skeleton className="h-4 w-16" /></TableCell>}
              {visibleColumns.memoryBar && <TableCell className="w-[15%]"><Skeleton className="h-4 w-full" /></TableCell>}
              {visibleColumns.tps && <TableCell className="w-px whitespace-nowrap px-2"><Skeleton className="h-4 w-16" /></TableCell>}
              {visibleColumns.tpsBar && <TableCell className="w-[15%]"><Skeleton className="h-4 w-full" /></TableCell>}
              {visibleColumns.errors && <TableCell className="w-px whitespace-nowrap"><Skeleton className="h-4 w-8 ml-auto" /></TableCell>}
              {visibleColumns.tags && <TableCell className="w-24 pr-4">
                <div className="flex gap-2">
                  <Skeleton className="h-5 w-12 rounded-full" />
                  <Skeleton className="h-5 w-10 rounded-full" />
                </div>
              </TableCell>}
              <TableCell className="w-[var(--spacing)] p-0"></TableCell>
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
    <div className="h-full">
      <Table containerClassName="h-full overflow-auto bg-[linear-gradient(90deg,var(--primary)_1px,transparent_1px)]" className="w-full text-xs">
        <TableHeader className="sticky top-0 z-20 bg-background">
          <TableRow className="border-b-primary border-b-2 hover:bg-transparent">
            {visibleColumns.rank && <TableHead className="w-8 pl-4">
              <div className="flex items-center justify-between">
                <span>#</span>
                {lastVisibleColumn === 'rank' && settingsMenu}
              </div>
            </TableHead>}
            {visibleColumns.framework && <TableHead className="w-[20%] pl-4">
              <div className="flex items-center justify-between">
                <span>Framework</span>
                {lastVisibleColumn === 'framework' && settingsMenu}
              </div>
            </TableHead>}
            {visibleColumns.rps && <TableHead className="w-auto">
              <div className="flex items-center justify-between">
                <span>Requests/sec</span>
                {lastVisibleColumn === 'rps' && settingsMenu}
              </div>
            </TableHead>}
            {visibleColumns.memory && <TableHead className="w-px whitespace-nowrap px-2">
              <div className="flex items-center justify-between">
                <span>Memory</span>
                {lastVisibleColumn === 'memory' && settingsMenu}
              </div>
            </TableHead>}
            {visibleColumns.memoryBar && <TableHead className="w-[15%]">
              <div className="flex items-center justify-between">
                <span></span>
                {lastVisibleColumn === 'memoryBar' && settingsMenu}
              </div>
            </TableHead>}
            {visibleColumns.tps && <TableHead className="w-px whitespace-nowrap px-2">
              <div className="flex items-center justify-between">
                <span>TPS</span>
                {lastVisibleColumn === 'tps' && settingsMenu}
              </div>
            </TableHead>}
            {visibleColumns.tpsBar && <TableHead className="w-[15%]">
              <div className="flex items-center justify-between">
                <span></span>
                {lastVisibleColumn === 'tpsBar' && settingsMenu}
              </div>
            </TableHead>}
            {visibleColumns.errors && <TableHead className="w-px text-right whitespace-nowrap">
              <div className="flex items-center justify-end">
                <span>Errors</span>
                {lastVisibleColumn === 'errors' && settingsMenu}
              </div>
            </TableHead>}
            {visibleColumns.tags && <TableHead className="w-24 pr-4">
              <div className="flex items-center justify-between">
                <span>Tags</span>
                {lastVisibleColumn === 'tags' && settingsMenu}
              </div>
            </TableHead>}
            <TableHead className="w-[var(--spacing)] p-0"></TableHead>
          </TableRow>
        </TableHeader>

        <TableBody>
          {sorted.map((benchmark, index) => {
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
                    {visibleColumns.rank && <TableCell className="w-8 pl-4 font-mono text-muted-foreground">
                      {index + 1}
                    </TableCell>}
                    {visibleColumns.framework && <TableCell className="w-[20%] pl-4">
                      <div className="flex items-center justify-between w-full">
                        <div className="flex items-center min-w-0">
                          <span
                            className="inline-block w-3 h-3 rounded-sm mr-2 shrink-0"
                            style={{ backgroundColor: langColor }}
                          />
                          <div className="font-medium flex items-center gap-x-2 gap-y-1 min-w-0">
                            <div className="flex items-center whitespace-nowrap">
                              {languageHref ? (
                                <a href={languageHref} target="_blank" rel="noopener noreferrer" className="text-foreground hover:underline hover:text-primary">
                                  {benchmark.language}
                                </a>
                              ) : (
                                <span>{benchmark.language}</span>
                              )}
                              <span className="mx-0.5 text-muted-foreground">/</span>
                              {frameworkHref ? (
                                <a href={frameworkHref} target="_blank" rel="noopener noreferrer" className="text-foreground hover:underline hover:text-primary">
                                  {benchmark.framework}
                                </a>
                              ) : (
                                <span>{benchmark.framework}</span>
                              )}
                            </div>
                            <span className="text-[10px] text-muted-foreground lowercase leading-none">v{benchmark.frameworkVersion}</span>
                            {benchmark.database && (
                              <span className={cn("inline-flex items-center rounded-md border px-1.5 py-0.5 text-[10px] font-semibold transition-colors focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 border-transparent h-5", getDatabaseColor(benchmark.database))}>
                                {benchmark.database}
                              </span>
                            )}
                          </div>
                        </div>
                        <span className="text-[10px] text-muted-foreground ml-2 truncate min-w-0 flex-1 text-right">
                          {benchmark.name}
                        </span>
                      </div>
                    </TableCell>}

                    {visibleColumns.rps && <TableCell className="w-auto">
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

                    {visibleColumns.memory && <TableCell className="w-px whitespace-nowrap px-2">
                      <span className="text-xs font-mono text-muted-foreground">
                        {(benchmark.memoryUsage / (1024 * 1024)).toFixed(1)}MB
                      </span>
                    </TableCell>}

                    {visibleColumns.memoryBar && <TableCell className="w-[15%]">
                      <div className="flex items-center gap-3">
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

                    {visibleColumns.tps && <TableCell className="w-px whitespace-nowrap px-2">
                      <span className="text-xs font-mono text-muted-foreground">
                        {formatNumber(benchmark.tps)}/s
                      </span>
                    </TableCell>}

                    {visibleColumns.tpsBar && <TableCell className="w-[15%]">
                      <div className="flex items-center gap-3">
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

                    {visibleColumns.errors && <TableCell className="w-px text-right whitespace-nowrap">
                      <span className={benchmark.errors === 0 ? 'text-muted-foreground' : 'text-red-500'}>
                        {benchmark.errors}
                      </span>
                    </TableCell>}

                    {visibleColumns.tags && <TableCell className="w-24 pr-4">
                      <TagsInline tags={benchmark.tags} />
                    </TableCell>}
                    <TableCell className="w-[var(--spacing)] p-0"></TableCell>
                  </TableRow>
                </HoverCardTrigger>
                <BenchmarkHoverDetails benchmark={benchmark} langColor={langColor} />
              </HoverCard>
            )
          })}
        </TableBody>

        <TableCaption>{sorted.length} results</TableCaption>
      </Table>
    </div>
  )
}
