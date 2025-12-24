import { useEffect, useState } from 'react'
import { useAppStore, type AppState } from '../store/useAppStore'
import { Skeleton } from '@/components/ui/skeleton'
import { getIcon } from '../lib/utils'
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"

export default function TestSelector() {
  const tests = useAppStore((s: AppState) => s.tests)
  const selectedTest = useAppStore((s: AppState) => s.selectedTest)
  const setSelectedTest = useAppStore((s: AppState) => s.setSelectedTest)
  const loadingTests = useAppStore((s: AppState) => s.testsLoading)
  const [localLoading, setLocalLoading] = useState(false)

  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | undefined

    if (loadingTests) {
      timer = setTimeout(() => setLocalLoading(true), 200)
    }

    return () => {
      if (timer) clearTimeout(timer)
      setLocalLoading(false)
    }
  }, [loadingTests])

  if (localLoading) {
    return (
      <div className="flex rounded-md border border-border bg-muted/50 p-1 gap-1">
        <Skeleton className="h-8 w-16" />
        <Skeleton className="h-8 w-20" />
        <Skeleton className="h-8 w-18" />
        <Skeleton className="h-8 w-14" />
      </div>
    )
  }

  return (
    <div className="flex rounded-md border border-border bg-muted/50 p-1">
      {tests.map((test) => {
        const Icon = getIcon(test.icon)
        return (
          <Tooltip key={test.id}>
            <TooltipTrigger asChild>
              <button
                className={`flex items-center gap-2 px-3 py-1.5 text-sm font-medium rounded-sm transition-colors ${
                  selectedTest === test.id
                    ? 'bg-background text-foreground shadow-sm'
                    : 'text-muted-foreground hover:text-foreground hover:bg-background/50'
                }`}
                onClick={() => setSelectedTest(test.id)}
              >
                <Icon className="h-4 w-4" />
                <span className="hidden min-[1750px]:inline">{test.name}</span>
              </button>
            </TooltipTrigger>
            <TooltipContent className="min-[1750px]:hidden">
              <p>{test.name}</p>
            </TooltipContent>
          </Tooltip>
        )
      })}
    </div>
  )
}