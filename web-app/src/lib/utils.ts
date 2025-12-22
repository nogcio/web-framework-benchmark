import { clsx, type ClassValue } from "clsx"
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

export function getColorForLanguage(lang: string) {
  let hash = 0
  for (let i = 0; i < lang.length; i++) {
    hash = (hash << 5) - hash + lang.charCodeAt(i)
    hash |= 0
  }
  const hue = Math.abs(hash) % 360
  return `hsl(${hue}, 65%, 50%)`
}

export function formatNumber(num: number): string {
  if (num >= 1_000_000) {
    return (num / 1_000_000).toFixed(1) + 'M'
  }
  if (num >= 1_000) {
    return (num / 1_000).toFixed(1) + 'k'
  }
  return num.toString()
}

export function getDatabaseColor(db: string) {
  switch (db.toLowerCase()) {
    case 'postgres':
    case 'postgresql':
    case 'pg':
      return 'bg-blue-100 text-blue-800 dark:bg-blue-900/50 dark:text-blue-300 hover:bg-blue-200 dark:hover:bg-blue-900'
    case 'mysql':
      return 'bg-cyan-100 text-cyan-800 dark:bg-cyan-900/50 dark:text-cyan-300 hover:bg-cyan-200 dark:hover:bg-cyan-900'
    case 'mssql':
    case 'sqlserver':
      return 'bg-red-100 text-red-800 dark:bg-red-900/50 dark:text-red-300 hover:bg-red-200 dark:hover:bg-red-900'
    case 'mongo':
    case 'mongodb':
      return 'bg-green-100 text-green-800 dark:bg-green-900/50 dark:text-green-300 hover:bg-green-200 dark:hover:bg-green-900'
    case 'redis':
      return 'bg-rose-100 text-rose-800 dark:bg-rose-900/50 dark:text-rose-300 hover:bg-rose-200 dark:hover:bg-rose-900'
    case 'sqlite':
      return 'bg-sky-100 text-sky-800 dark:bg-sky-900/50 dark:text-sky-300 hover:bg-sky-200 dark:hover:bg-sky-900'
    default:
      return 'bg-secondary text-secondary-foreground hover:bg-secondary/80'
  }
}
