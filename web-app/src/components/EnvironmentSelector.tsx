import { useAppStore, type AppState } from '../store/useAppStore'
import { getIcon } from '../lib/utils'
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"

export default function EnvironmentSelector() {
  const environments = useAppStore((s: AppState) => s.environments)
  const selectedEnvironment = useAppStore((s: AppState) => s.selectedEnvironment)
  const setSelectedEnvironment = useAppStore((s: AppState) => s.setSelectedEnvironment)

  return (
    <div className="flex rounded-lg border border-border bg-muted/50 p-1">
      {environments.map((env) => {
        const Icon = getIcon(env.icon)
        const isSelected = selectedEnvironment === env.name
        return (
          <Tooltip key={env.name}>
            <TooltipTrigger asChild>
              <button
                className={`flex items-center gap-2 px-3 py-1.5 text-sm font-medium rounded-md transition-all ${
                  isSelected
                    ? 'bg-background text-foreground shadow-sm ring-1 ring-black/5 dark:ring-white/10'
                    : 'text-muted-foreground hover:text-foreground hover:bg-background/50'
                }`}
                onClick={() => setSelectedEnvironment(env.name)}
              >
                <Icon className="h-4 w-4" />
                <span className="hidden min-[1750px]:inline">{env.displayName}</span>
              </button>
            </TooltipTrigger>
            <TooltipContent className="min-[1750px]:hidden">
              <p>{env.displayName}</p>
            </TooltipContent>
          </Tooltip>
        )
      })}
    </div>
  )
}
