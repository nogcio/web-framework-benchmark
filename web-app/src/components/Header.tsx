import { Home, Server, Laptop, Cloud, Activity } from 'lucide-react'
import { useAppStore, type AppState } from '../store/useAppStore'
import TestSelector from './TestSelector'

const iconMap: Record<string, React.ElementType> = {
  home: Home,
  server: Server,
  laptop: Laptop,
  cloud: Cloud,
}

export default function Header() {
  const environments = useAppStore((s: AppState) => s.environments)
  const selectedEnvironment = useAppStore((s: AppState) => s.selectedEnvironment)
  const setSelectedEnvironment = useAppStore((s: AppState) => s.setSelectedEnvironment)

  return (
    <div className="-mx-4 -mt-4 mb-6">
      <div className="border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60 px-6 py-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="flex h-10 w-10 items-center justify-center rounded-xl bg-primary/10 text-primary shadow-sm">
              <Activity className="h-6 w-6" />
            </div>
            <div className="flex flex-col gap-0.5">
              <h1 className="text-xl font-bold tracking-tight">
                Web Framework <span className="text-primary">Benchmarks</span>
              </h1>
              <p className="text-[10px] text-muted-foreground font-medium uppercase tracking-wider">
                Performance Analysis Tool
              </p>
            </div>
          </div>
          
          <div className="flex-1 flex justify-center px-8">
            <TestSelector />
          </div>

          <div className="flex items-center gap-2">
            <div className="flex rounded-lg border border-border bg-muted/50 p-1">
              {environments.map((env) => {
                const Icon = iconMap[env.icon] || Server
                const isSelected = selectedEnvironment === env.name
                return (
                  <button
                    key={env.name}
                    className={`flex items-center gap-2 px-3 py-1.5 text-sm font-medium rounded-md transition-all ${
                      isSelected
                        ? 'bg-background text-foreground shadow-sm ring-1 ring-black/5 dark:ring-white/10'
                        : 'text-muted-foreground hover:text-foreground hover:bg-background/50'
                    }`}
                    onClick={() => setSelectedEnvironment(env.name)}
                  >
                    <Icon className="h-4 w-4" />
                    <span>{env.displayName}</span>
                  </button>
                )
              })}
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
