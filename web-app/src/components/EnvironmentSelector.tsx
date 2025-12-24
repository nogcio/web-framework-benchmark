import { useAppStore, type AppState } from '../store/useAppStore'
import { getIcon } from '../lib/utils'
import { Button } from './ui/button'
import { createElement } from 'react'
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

export default function EnvironmentSelector() {
  const environments = useAppStore((s: AppState) => s.environments)
  const selectedEnvironment = useAppStore((s: AppState) => s.selectedEnvironment)
  const setSelectedEnvironment = useAppStore((s: AppState) => s.setSelectedEnvironment)

  const currentEnv = environments.find(e => e.name === selectedEnvironment)
  const CurrentIcon = currentEnv ? getIcon(currentEnv.icon) : null

  if (!currentEnv || !CurrentIcon) return null

  return (
    <DropdownMenu>
      <Tooltip>
        <TooltipTrigger asChild>
          <DropdownMenuTrigger asChild>
            <Button variant="outline" size="sm" className="gap-2">
              {createElement(CurrentIcon, { className: "h-4 w-4" })}
              <span className="hidden 2xl:inline font-semibold">{currentEnv.displayName}</span>
              <ChevronDown className="h-3 w-3 opacity-50" />
            </Button>
          </DropdownMenuTrigger>
        </TooltipTrigger>
        <TooltipContent>
          <p>{currentEnv.displayName}</p>
        </TooltipContent>
      </Tooltip>
      <DropdownMenuContent align="end">
        {environments.map((env) => {
          const Icon = getIcon(env.icon)
          return (
            <DropdownMenuItem 
              key={env.name} 
              onClick={() => setSelectedEnvironment(env.name)}
              className="gap-2"
            >
              {createElement(Icon, { className: "h-4 w-4" })}
              <span>{env.displayName}</span>
            </DropdownMenuItem>
          )
        })}
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
