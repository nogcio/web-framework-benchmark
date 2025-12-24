import { useMemo, useState, useEffect } from 'react'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow, TableCaption } from './ui/table'
import { Skeleton } from './ui/skeleton'
import { Empty, EmptyTitle, EmptyDescription, EmptyMedia } from './ui/empty'
import { HoverCard, HoverCardTrigger, HoverCardContent } from './ui/hover-card'
import { BarChart3, ChevronDown } from 'lucide-react'
import type { Benchmark, VisibleColumns, Test } from '../types'
import { useAppStore, type AppState } from '../store/useAppStore'
import { TagsInline } from './TagsInline'
import { getColorForLanguage, formatNumber, cn, getDatabaseColor } from '../lib/utils'
import { TableSettings } from './TableSettings'
import { BenchmarkHoverDetails } from './BenchmarkHoverDetails'
import { REPO_URL } from '../lib/constants'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
  DropdownMenuSub,
  DropdownMenuSubTrigger,
  DropdownMenuSubContent,
} from "./ui/dropdown-menu"

interface Props {
  benchmarks: Benchmark[]
}

export default function BenchmarksTable({ benchmarks }: Props) {
  const languages = useAppStore((s: AppState) => s.languages)
  const frameworks = useAppStore((s: AppState) => s.frameworks)
  const benchmarksLoading = useAppStore((s: AppState) => s.benchmarksLoading)
  const visibleColumns = useAppStore((s: AppState) => s.visibleColumns)
  const toggleColumn = useAppStore((s: AppState) => s.toggleColumn)
  
  const runs = useAppStore((s: AppState) => s.runs)
  const selectedRunId = useAppStore((s: AppState) => s.selectedRunId)
  const setSelectedRunId = useAppStore((s: AppState) => s.setSelectedRunId)
  
  const selectedTestId = useAppStore((s: AppState) => s.selectedTest)
  const setSelectedTest = useAppStore((s: AppState) => s.setSelectedTest)
  const tests = useAppStore((s: AppState) => s.tests)

  // Helper to find test recursively
  const findTest = (id: string | null, list: Test[]): Test | undefined => {
    if (!id) return undefined
    for (const t of list) {
      if (t.id === id) return t
      if (t.children) {
        const found = findTest(id, t.children)
        if (found) return found
      }
    }
    return undefined
  }

  const selectedTest = findTest(selectedTestId, tests)

  const [localLoading, setLocalLoading] = useState(false)

  const mobileHeader = (
    <div className="md:hidden px-4 py-2 border-b bg-muted/20 text-xs font-medium flex items-center gap-2 text-muted-foreground">
      <DropdownMenu>
        <DropdownMenuTrigger className="flex items-center gap-1 hover:text-foreground transition-colors outline-none">
          <span>Run #{selectedRunId}</span>
          <ChevronDown className="h-3 w-3 opacity-50" />
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start" className="max-h-[300px]">
          {[...runs].sort((a, b) => b.id - a.id).map((run) => (
            <DropdownMenuItem 
              key={run.id} 
              onClick={() => setSelectedRunId(run.id)}
              className={selectedRunId === run.id ? "bg-accent" : ""}
            >
              Run #{run.id} <span className="ml-2 text-muted-foreground text-[10px]">{new Date(run.createdAt).toLocaleDateString()}</span>
            </DropdownMenuItem>
          ))}
        </DropdownMenuContent>
      </DropdownMenu>

      <span>•</span>

      <DropdownMenu>
        <DropdownMenuTrigger className="flex items-center gap-1 hover:text-foreground transition-colors outline-none">
          <span>{selectedTest?.name || 'Select Test'}</span>
          <ChevronDown className="h-3 w-3 opacity-50" />
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start">
          {tests.map((test) => {
            if (test.children && test.children.length > 0) {
              return (
                <DropdownMenuSub key={test.name}>
                  <DropdownMenuSubTrigger>
                    <span>{test.name}</span>
                  </DropdownMenuSubTrigger>
                  <DropdownMenuSubContent>
                    {test.children.map((child) => (
                      <DropdownMenuItem 
                        key={child.id} 
                        onClick={() => child.id && setSelectedTest(child.id)}
                        className={selectedTestId === child.id ? "bg-accent" : ""}
                      >
                        {child.name}
                      </DropdownMenuItem>
                    ))}
                  </DropdownMenuSubContent>
                </DropdownMenuSub>
              )
            }
            return (
              <DropdownMenuItem 
                key={test.id} 
                onClick={() => test.id && setSelectedTest(test.id)}
                className={selectedTestId === test.id ? "bg-accent" : ""}
              >
                {test.name}
              </DropdownMenuItem>
            )
          })}
        </DropdownMenuContent>
      </DropdownMenu>
    </div>
  )

  useEffect(() => {
    if (benchmarksLoading) {
      const timer = setTimeout(() => setLocalLoading(true), 200)
      return () => clearTimeout(timer)
    } else {
      setTimeout(() => setLocalLoading(false), 0)
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
      <div className="h-full flex flex-col">
        {mobileHeader}
        <div className="flex-1 min-h-0">
          <Table containerClassName="h-full overflow-auto md:bg-[linear-gradient(90deg,var(--primary)_1px,transparent_1px)]" className="w-full text-xs">
            <TableHeader className="sticky top-0 z-20 bg-background">
              <TableRow>
                {visibleColumns.rank && <TableHead className="w-8 pl-4 hidden md:table-cell">#</TableHead>}
                {visibleColumns.framework && <TableHead className="w-full md:w-[20%] pl-4">Framework</TableHead>}
                {visibleColumns.rps && <TableHead className="w-auto">Requests/sec</TableHead>}
                {visibleColumns.memory && <TableHead className="w-px whitespace-nowrap px-2 hidden md:table-cell">Memory</TableHead>}
                {visibleColumns.memoryBar && <TableHead className="w-[15%] hidden md:table-cell"></TableHead>}
                {visibleColumns.tps && <TableHead className="w-px whitespace-nowrap px-2 hidden md:table-cell">TPS</TableHead>}
                {visibleColumns.tpsBar && <TableHead className="w-[15%] hidden md:table-cell"></TableHead>}
                {visibleColumns.errors && <TableHead className="w-px text-right whitespace-nowrap hidden md:table-cell">Errors</TableHead>}
                {visibleColumns.tags && <TableHead className="w-24 pr-4 hidden md:table-cell">Tags</TableHead>}
                <TableHead className="w-[var(--spacing)] p-0"></TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {Array.from({ length: 6 }).map((_, i) => (
                <TableRow key={i}>
                  {visibleColumns.rank && <TableCell className="w-8 pl-4 hidden md:table-cell"><Skeleton className="h-4 w-4" /></TableCell>}
                  {visibleColumns.framework && <TableCell className="w-full md:w-[20%] pl-4">
                    <div className="flex items-center">
                      <Skeleton className="w-3 h-3 rounded-sm mr-2" />
                      <Skeleton className="h-4 w-20" />
                    </div>
                  </TableCell>}
                  {visibleColumns.rps && <TableCell className="w-auto">
                    <div className="flex items-center gap-3">
                      <Skeleton className="w-16 h-3 ml-auto" />
                      <Skeleton className="flex-1 h-2 rounded hidden md:block" />
                      <Skeleton className="w-12 h-3 hidden md:block" />
                    </div>
                  </TableCell>}
                  {visibleColumns.memory && <TableCell className="w-px whitespace-nowrap px-2 hidden md:table-cell"><Skeleton className="h-4 w-16" /></TableCell>}
                  {visibleColumns.memoryBar && <TableCell className="w-[15%] hidden md:table-cell"><Skeleton className="h-4 w-full" /></TableCell>}
                  {visibleColumns.tps && <TableCell className="w-px whitespace-nowrap px-2 hidden md:table-cell"><Skeleton className="h-4 w-16" /></TableCell>}
                  {visibleColumns.tpsBar && <TableCell className="w-[15%] hidden md:table-cell"><Skeleton className="h-4 w-full" /></TableCell>}
                  {visibleColumns.errors && <TableCell className="w-px whitespace-nowrap hidden md:table-cell"><Skeleton className="h-4 w-8 ml-auto" /></TableCell>}
                  {visibleColumns.tags && <TableCell className="w-24 pr-4 hidden md:table-cell">
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
        </div>
      </div>
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
    <div className="h-full flex flex-col">
      {mobileHeader}
      <div className="flex-1 min-h-0">
        <Table containerClassName="h-full overflow-auto md:bg-[linear-gradient(90deg,var(--primary)_1px,transparent_1px)]" className="w-full text-xs">
        <TableHeader className="sticky top-0 z-20 bg-background">
          <TableRow className="border-b-primary border-b-2 hover:bg-transparent">
            {visibleColumns.rank && <TableHead className="w-8 pl-4 hidden md:table-cell">
              <div className="flex items-center justify-between">
                <span>#</span>
                {lastVisibleColumn === 'rank' && settingsMenu}
              </div>
            </TableHead>}
            {visibleColumns.framework && <TableHead className="w-full md:w-[20%] pl-4">
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
            {visibleColumns.memory && <TableHead className="w-px whitespace-nowrap px-2 hidden md:table-cell">
              <div className="flex items-center justify-between">
                <span>Memory</span>
                {lastVisibleColumn === 'memory' && settingsMenu}
              </div>
            </TableHead>}
            {visibleColumns.memoryBar && <TableHead className="w-[15%] hidden md:table-cell">
              <div className="flex items-center justify-between">
                <span></span>
                {lastVisibleColumn === 'memoryBar' && settingsMenu}
              </div>
            </TableHead>}
            {visibleColumns.tps && <TableHead className="w-px whitespace-nowrap px-2 hidden md:table-cell">
              <div className="flex items-center justify-between">
                <span>TPS</span>
                {lastVisibleColumn === 'tps' && settingsMenu}
              </div>
            </TableHead>}
            {visibleColumns.tpsBar && <TableHead className="w-[15%] hidden md:table-cell">
              <div className="flex items-center justify-between">
                <span></span>
                {lastVisibleColumn === 'tpsBar' && settingsMenu}
              </div>
            </TableHead>}
            {visibleColumns.errors && <TableHead className="w-px text-right whitespace-nowrap hidden md:table-cell">
              <div className="flex items-center justify-end">
                <span>Errors</span>
                {lastVisibleColumn === 'errors' && settingsMenu}
              </div>
            </TableHead>}
            {visibleColumns.tags && <TableHead className="w-24 pr-4 hidden md:table-cell">
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
            // Resolve language/framework URLs from the global store lists
            const language = languages.find((l) => l.name === benchmark.language)
            const framework = frameworks.find((f) => f.name === benchmark.framework)
            
            const langColor = language?.color || getColorForLanguage(benchmark.language || 'unknown')
            
            const languageHref = language?.url
            const frameworkHref = framework?.url
            const rpsPercent = maxRps > 0 ? (benchmark.rps / maxRps) * 100 : 0
            
            // Add opacity to the color for the background row
            const rowColor = langColor.startsWith('hsl') 
              ? langColor.replace(')', ', 0.2)') 
              : langColor

            return (
              <HoverCard key={benchmark.name} openDelay={300} closeDelay={300}>
                <HoverCardTrigger asChild>
                  <TableRow
                    className="hover:cursor-pointer bg-[linear-gradient(to_right,var(--row-color)_var(--row-progress),transparent_var(--row-progress))] md:bg-none"
                    style={{
                      '--row-progress': `${rpsPercent}%`,
                      '--row-color': rowColor
                    } as React.CSSProperties}
                    onClick={() => {
                      if (benchmark.path) {
                        window.open(`${REPO_URL}/tree/main/${benchmark.path}`, '_blank')
                      }
                    }}
                  >
                    {visibleColumns.rank && <TableCell className="w-8 pl-4 font-mono text-muted-foreground hidden md:table-cell">
                      {index + 1}
                    </TableCell>}
                    {visibleColumns.framework && <TableCell className="w-full md:w-[20%] pl-4">
                      <div className="flex items-center justify-between w-full">
                        <div className="flex items-center min-w-0 hidden md:flex">
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
                        <div className="flex items-center min-w-0 flex-1 md:justify-end">
                          <span
                            className="inline-block w-2 h-2 rounded-full mr-2 shrink-0 md:hidden"
                            style={{ backgroundColor: langColor }}
                          />
                          <span className="text-sm md:text-[10px] md:text-muted-foreground md:ml-2 truncate min-w-0 text-left md:text-right font-medium md:font-normal">
                            {benchmark.name}
                          </span>
                        </div>
                      </div>
                    </TableCell>}

                    {visibleColumns.rps && <TableCell className="w-auto">
                      <div className="flex items-center gap-3">
                        <div className="w-20 text-right text-xs font-mono shrink-0">{Math.round(benchmark.rps || 0).toLocaleString()}</div>
                        <div className="flex-1 hidden md:block">
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
                        <div className="w-12 text-left text-[10px] font-mono text-muted-foreground leading-none hidden md:block">
                          {(() => {
                            const rpsPercent = maxRps > 0 ? (benchmark.rps / maxRps) * 100 : 0
                            return `${Math.round(rpsPercent)}%`
                          })()}
                        </div>
                      </div>
                    </TableCell>}

                    {visibleColumns.memory && <TableCell className="w-px whitespace-nowrap px-2 hidden md:table-cell">
                      <span className="text-xs font-mono text-muted-foreground">
                        {(benchmark.memoryUsage / (1024 * 1024)).toFixed(1)}MB
                      </span>
                    </TableCell>}

                    {visibleColumns.memoryBar && <TableCell className="w-[15%] hidden md:table-cell">
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

                    {visibleColumns.tps && <TableCell className="w-px whitespace-nowrap px-2 hidden md:table-cell">
                      <span className="text-xs font-mono text-muted-foreground">
                        {formatNumber(benchmark.tps)}/s
                      </span>
                    </TableCell>}

                    {visibleColumns.tpsBar && <TableCell className="w-[15%] hidden md:table-cell">
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

                    {visibleColumns.errors && <TableCell className="w-px text-right whitespace-nowrap hidden md:table-cell">
                      <span className={benchmark.errors === 0 ? 'text-muted-foreground' : 'text-red-500'}>
                        {benchmark.errors}
                      </span>
                    </TableCell>}

                    {visibleColumns.tags && <TableCell className="w-24 pr-4 hidden md:table-cell">
                      <TagsInline tags={benchmark.tags} />
                    </TableCell>}
                    <TableCell className="w-[var(--spacing)] p-0"></TableCell>
                  </TableRow>
                </HoverCardTrigger>
                <HoverCardContent className="w-auto p-0">
                  <BenchmarkHoverDetails benchmark={benchmark} langColor={langColor} />
                </HoverCardContent>
              </HoverCard>
            )
          })}
        </TableBody>

        <TableCaption>{sorted.length} results</TableCaption>
      </Table>
      </div>
    </div>
  )
}
