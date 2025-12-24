import { useEffect, useState, createElement } from 'react'
import { useAppStore, type AppState } from '../store/useAppStore'
import { Skeleton } from '@/components/ui/skeleton'
import { getIcon } from '../lib/utils'
import { Button } from './ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip"
import { ChevronDown } from 'lucide-react'
import type { Test } from '../types'

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

  const renderTestItem = (test: Test) => {
    const Icon = getIcon(test.icon)
    const hasChildren = test.children && test.children.length > 0
    
    // Check if this test or any of its children is selected
    const isSelected = test.id === selectedTest
    const isChildSelected = hasChildren && test.children?.some(child => child.id === selectedTest)
    
    if (hasChildren) {
        const selectedChild = test.children?.find(child => child.id === selectedTest)
        const label = isChildSelected && selectedChild ? selectedChild.name : test.name
        
        return (
            <DropdownMenu key={test.name}>
                <Tooltip>
                    <TooltipTrigger asChild>
                        <DropdownMenuTrigger asChild>
                            <Button 
                                variant="ghost" 
                                size="sm" 
                                className={`gap-2 h-8 px-3 ${isChildSelected ? 'bg-background shadow-sm text-foreground hover:bg-background' : 'text-muted-foreground hover:text-foreground hover:bg-background/50'}`}
                            >
                                {createElement(Icon, { className: "h-4 w-4" })}
                                <span className="hidden 2xl:inline font-medium">
                                    {label}
                                </span>
                                <ChevronDown className="h-3 w-3 opacity-50" />
                            </Button>
                        </DropdownMenuTrigger>
                    </TooltipTrigger>
                    <TooltipContent>
                        <p>{label}</p>
                    </TooltipContent>
                </Tooltip>
                <DropdownMenuContent align="start">
                    {test.children?.map(child => {
                        const ChildIcon = getIcon(child.icon)
                        return (
                            <DropdownMenuItem 
                                key={child.id} 
                                onClick={() => child.id && setSelectedTest(child.id)}
                                className="gap-2"
                            >
                                {createElement(ChildIcon, { className: "h-4 w-4" })}
                                <span>{child.name}</span>
                            </DropdownMenuItem>
                        )
                    })}
                </DropdownMenuContent>
            </DropdownMenu>
        )
    }

    return (
        <Tooltip key={test.id}>
            <TooltipTrigger asChild>
                <Button
                    variant="ghost"
                    size="sm"
                    className={`gap-2 h-8 px-3 ${isSelected ? 'bg-background shadow-sm text-foreground hover:bg-background' : 'text-muted-foreground hover:text-foreground hover:bg-background/50'}`}
                    onClick={() => test.id && setSelectedTest(test.id)}
                >
                    {createElement(Icon, { className: "h-4 w-4" })}
                    <span className="hidden 2xl:inline font-medium">{test.name}</span>
                </Button>
            </TooltipTrigger>
            <TooltipContent>
                <p>{test.name}</p>
            </TooltipContent>
        </Tooltip>
    )
  }

  return (
    <div className="flex rounded-lg border border-border bg-muted/50 p-1 gap-1">
      {tests.map(renderTestItem)}
    </div>
  )
}