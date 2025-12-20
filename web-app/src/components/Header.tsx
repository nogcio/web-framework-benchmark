import React from 'react'
import { Button } from './ui/button'
import { Moon, Sun } from 'lucide-react'
import { useAppStore, type AppState } from '../store/useAppStore'

export default function Header() {
  const theme = useAppStore((s: AppState) => s.theme)
  const toggleTheme = useAppStore((s: AppState) => s.toggleTheme)

  return (
    <div className="-mx-4 -mt-4 mb-4">
      <div className="flex justify-between items-center shadow-md border-b border-gray-200 dark:border-gray-700 p-4 bg-gradient-to-b from-white/80 to-white/60 dark:from-zinc-800/80 dark:to-zinc-800/60 backdrop-blur-sm">
        <h1 className="text-2xl font-bold">Web Framework Benchmarks</h1>
        <Button variant="outline" size="icon-sm" className="p-0.5 h-6 w-6" onClick={toggleTheme}>
          {theme === 'dark' ? <Moon className="h-2.5 w-2.5" /> : <Sun className="h-2.5 w-2.5" />}
        </Button>
      </div>
    </div>
  )
}
