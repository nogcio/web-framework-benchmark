import TestSelector from './TestSelector'
import EnvironmentSelector from './EnvironmentSelector'
import { SidebarTrigger } from './ui/sidebar'

export default function Header() {
  return (
    <div className="-mx-4 -mt-4 mb-0 md:mb-6 relative z-50">
      <div className="border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60 px-4 md:px-6 py-3">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <img 
              src="/logo.svg" 
              alt="WFB Logo" 
              className="h-8 w-8 md:h-10 md:w-10 rounded-xl shadow-sm"
            />
            <div className="flex flex-col gap-0.5">
              <h1 className="text-lg md:text-xl font-bold tracking-tight">
                <span className="hidden lg:inline">Web Framework </span><span className="text-primary">Benchmarks</span>
              </h1>
              <p className="hidden md:block text-[10px] text-muted-foreground font-medium uppercase tracking-wider">
                Performance Analysis Tool
              </p>
            </div>
          </div>
          
          {/* Desktop Navigation */}
          <div className="hidden md:flex flex-1 justify-center px-8">
            <TestSelector />
          </div>

          <div className="hidden md:flex items-center gap-2">
            <EnvironmentSelector />
          </div>

          {/* Mobile Navigation */}
          <div className="md:hidden">
            <SidebarTrigger />
          </div>
        </div>
      </div>
    </div>
  )
}
