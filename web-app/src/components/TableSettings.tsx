import { Button } from './ui/button'
import { DropdownMenu, DropdownMenuContent, DropdownMenuTrigger, DropdownMenuCheckboxItem } from './ui/dropdown-menu'
import { Settings } from 'lucide-react'
import type { VisibleColumns } from '../types'

interface TableSettingsProps {
  visibleColumns: VisibleColumns
  onToggleColumn: (column: keyof VisibleColumns) => void
}

export function TableSettings({ visibleColumns, onToggleColumn }: TableSettingsProps) {
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="icon" className="h-6 w-6 ml-1 mb-1">
          <Settings className="h-3 w-3" />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuCheckboxItem
          checked={visibleColumns.rps}
          onCheckedChange={() => onToggleColumn('rps')}
        >
          Rps
        </DropdownMenuCheckboxItem>
        <DropdownMenuCheckboxItem
          checked={visibleColumns.memory}
          onCheckedChange={() => onToggleColumn('memory')}
        >
          Memory
        </DropdownMenuCheckboxItem>
        <DropdownMenuCheckboxItem
          checked={visibleColumns.memoryBar}
          onCheckedChange={() => onToggleColumn('memoryBar')}
        >
          Memory Bar
        </DropdownMenuCheckboxItem>
        <DropdownMenuCheckboxItem
          checked={visibleColumns.tps}
          onCheckedChange={() => onToggleColumn('tps')}
        >
          TPS
        </DropdownMenuCheckboxItem>
        <DropdownMenuCheckboxItem
          checked={visibleColumns.tpsBar}
          onCheckedChange={() => onToggleColumn('tpsBar')}
        >
          TPS Bar
        </DropdownMenuCheckboxItem>
        <DropdownMenuCheckboxItem
          checked={visibleColumns.errors}
          onCheckedChange={() => onToggleColumn('errors')}
        >
          Errors
        </DropdownMenuCheckboxItem>
        <DropdownMenuCheckboxItem
          checked={visibleColumns.tags}
          onCheckedChange={() => onToggleColumn('tags')}
        >
          Tags
        </DropdownMenuCheckboxItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
