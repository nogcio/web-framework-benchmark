import { Button } from './ui/button'
import { Moon, Sun, Home, Server } from 'lucide-react'
import { useAppStore, type AppState } from '../store/useAppStore'
import TestSelector from './TestSelector'

export default function Header() {
  const theme = useAppStore((s: AppState) => s.theme)
  const toggleTheme = useAppStore((s: AppState) => s.toggleTheme)
  const environments = useAppStore((s: AppState) => s.environments)
  const selectedEnvironment = useAppStore((s: AppState) => s.selectedEnvironment)
  const setSelectedEnvironment = useAppStore((s: AppState) => s.setSelectedEnvironment)

  return (
    <div className="-mx-4 -mt-4">
      <div className="shadow-md p-4">
        <div className="flex items-center mb-4">
          <h1 className="text-2xl font-bold">Web Framework Benchmarks</h1>
          <div className="flex-1 flex justify-center">
            <TestSelector />
          </div>
          <div className="flex items-center gap-2">
            <div className="flex rounded-md border border-border bg-muted/50 p-1">
              {environments.map((env) => {
                const Icon = env.icon === 'home' ? Home : Server
                const isSelected = selectedEnvironment === env.name
                return (
                  <button
                    key={env.name}
                    className={`flex items-center gap-2 px-3 py-1.5 text-sm font-medium rounded-sm transition-colors ${
                      isSelected
                        ? 'bg-background text-foreground shadow-sm'
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
            <Button variant="outline" size="icon-sm" className="p-1 h-8 w-8 rounded-sm" onClick={toggleTheme}>
              {theme === 'dark' ? <Moon className="h-4 w-4" /> : <Sun className="h-4 w-4" />}
            </Button>
          </div>
        </div>
      </div>
    </div>
  )
}
