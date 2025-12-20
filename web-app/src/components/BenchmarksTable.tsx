import React, { useMemo, useState } from 'react'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow, TableCaption } from './ui/table'
import type { Benchmark } from '../types'
import { useAppStore, type AppState } from '../store/useAppStore'

interface Props {
  benchmarks: Benchmark[]
}

export default function BenchmarksTable({ benchmarks }: Props) {
  const languages = useAppStore((s: AppState) => s.languages)
  const frameworks = useAppStore((s: AppState) => s.frameworks)
  const sorted = useMemo(() => {
    return [...benchmarks].sort((a, b) => b.requests_per_second - a.requests_per_second)
  }, [benchmarks])

  const maxRps = useMemo(() => {
    if (!benchmarks || benchmarks.length === 0) return 0
    return Math.max(...benchmarks.map((b) => b.requests_per_second))
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

  return (
    <Table className="w-full text-xs">
      <TableHeader>
        <TableRow>
          <TableHead className="w-1/6 pl-4">Framework</TableHead>
          <TableHead className="w-1/6 text-right">Requests/sec</TableHead>
          <TableHead className="w-1/2"></TableHead>
          <TableHead className="w-1/6 pr-4">Tags</TableHead>
        </TableRow>
      </TableHeader>

      <TableBody>
        {sorted.map((benchmark) => {
          const langColor = getColorForLanguage(benchmark.language_id)
          // Resolve language/framework URLs from the global store lists
          const language = languages.find((l) => l.id === benchmark.language_id)
          const framework = frameworks.find((f) => f.id === benchmark.framework_id)
          const languageHref = language?.link
          const frameworkHref = framework?.repo_link

          return (
            <TableRow key={benchmark.id}>
              <TableCell className="w-1/6 pl-4">
                <div className="flex items-center">
                  <span
                    className="inline-block w-3 h-3 rounded-sm mr-2"
                    style={{ backgroundColor: langColor }}
                  />
                  <div className="font-medium">
                    {languageHref ? (
                      <a href={languageHref} target="_blank" rel="noopener noreferrer" className="text-primary hover:underline">
                        {benchmark.language_id}
                      </a>
                    ) : (
                      <span>{benchmark.language_id}</span>
                    )}
                    <span className="mx-1 text-muted-foreground">/</span>
                    {frameworkHref ? (
                      <a href={frameworkHref} target="_blank" rel="noopener noreferrer" className="text-primary hover:underline">
                        {benchmark.framework_id}
                      </a>
                    ) : (
                      <span>{benchmark.framework_id}</span>
                    )}
                    <span className="text-[10px] text-muted-foreground lowercase ml-2 leading-none">v{benchmark.version}</span>
                  </div>
                </div>
              </TableCell>

              <TableCell className="w-1/6 text-right">
                <div className="w-28 ml-auto font-mono">{benchmark.requests_per_second.toLocaleString()}</div>
              </TableCell>

              <TableCell className="w-1/2">
                <div className="flex items-center gap-3">
                  <div className="flex-1">
                    <div className="w-full h-2 bg-gray-200 dark:bg-gray-700 rounded overflow-hidden">
                      <div
                        className="h-2"
                        style={{
                          width: `${maxRps > 0 ? (benchmark.requests_per_second / maxRps) * 100 : 0}%`,
                          backgroundColor: langColor,
                        }}
                      />
                    </div>
                  </div>

                  <div className="w-12 text-right text-[10px] font-mono text-muted-foreground leading-none">
                    {maxRps > 0 ? `${Math.round((benchmark.requests_per_second / maxRps) * 100)}%` : '0%'}
                  </div>
                </div>
              </TableCell>

              <TableCell className="w-1/6 pr-4">
                <TagsInline tags={benchmark.tags} />
              </TableCell>
            </TableRow>
          )
        })}
      </TableBody>

      <TableCaption>{sorted.length} results</TableCaption>
    </Table>
  )
}
