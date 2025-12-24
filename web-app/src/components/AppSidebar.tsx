import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarMenuSub,
  SidebarMenuSubButton,
  SidebarMenuSubItem,
  useSidebar,
} from "@/components/ui/sidebar"
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible"
import { useAppStore, type AppState } from '../store/useAppStore'
import { ChevronDown, FlaskConical, History, Server } from 'lucide-react'
import { getIcon } from '../lib/utils'
import { createElement } from 'react'

export function AppSidebar() {
  const { setOpenMobile } = useSidebar()
  const tests = useAppStore((s: AppState) => s.tests)
  const selectedTest = useAppStore((s: AppState) => s.selectedTest)
  const setSelectedTest = useAppStore((s: AppState) => s.setSelectedTest)
  
  const environments = useAppStore((s: AppState) => s.environments)
  const selectedEnvironmentName = useAppStore((s: AppState) => s.selectedEnvironment)
  const setSelectedEnvironment = useAppStore((s: AppState) => s.setSelectedEnvironment)

  const runs = useAppStore((s: AppState) => s.runs)
  const selectedRunId = useAppStore((s: AppState) => s.selectedRunId)
  const setSelectedRunId = useAppStore((s: AppState) => s.setSelectedRunId)
  const sortedRuns = [...runs].sort((a, b) => b.id - a.id)

  const selectedEnvironment = environments.find(e => e.name === selectedEnvironmentName)

  const handleSelection = (action: () => void) => {
    action()
    setOpenMobile(false)
  }

  return (
    <Sidebar className="md:hidden">
      <SidebarHeader>
        <div className="flex items-center gap-3 px-2 py-2">
          <img 
            src="/logo.svg" 
            alt="WFB Logo" 
            className="h-8 w-8 rounded-xl shadow-sm"
          />
          <h1 className="flex items-baseline text-lg font-bold tracking-tight leading-none">
            <span>WF</span>
            <span className="text-primary">B</span>
          </h1>
        </div>
      </SidebarHeader>
      <SidebarContent>
        <Collapsible defaultOpen className="group/collapsible">
          <SidebarGroup>
            <SidebarGroupLabel asChild>
              <CollapsibleTrigger>
                <FlaskConical className="mr-2 h-4 w-4" />
                Test
                <ChevronDown className="ml-auto transition-transform group-data-[state=open]/collapsible:rotate-180" />
              </CollapsibleTrigger>
            </SidebarGroupLabel>
            <CollapsibleContent>
              <SidebarGroupContent>
                <SidebarMenu>
                  {tests.map((test) => {
                    const Icon = getIcon(test.icon)
                    
                    if (test.children && test.children.length > 0) {
                      return (
                        <Collapsible key={test.name} asChild defaultOpen className="group/sub">
                          <SidebarMenuItem>
                            <CollapsibleTrigger asChild>
                              <SidebarMenuButton tooltip={test.name}>
                                {createElement(Icon)}
                                <span>{test.name}</span>
                                <ChevronDown className="ml-auto transition-transform group-data-[state=open]/sub:rotate-180" />
                              </SidebarMenuButton>
                            </CollapsibleTrigger>
                            <CollapsibleContent>
                              <SidebarMenuSub>
                                {test.children.map((child) => {
                                  const ChildIcon = getIcon(child.icon)
                                  return (
                                    <SidebarMenuSubItem key={child.id}>
                                      <SidebarMenuSubButton
                                        isActive={selectedTest === child.id}
                                        onClick={() => handleSelection(() => child.id && setSelectedTest(child.id))}
                                      >
                                        {createElement(ChildIcon)}
                                        <span>{child.name}</span>
                                      </SidebarMenuSubButton>
                                    </SidebarMenuSubItem>
                                  )
                                })}
                              </SidebarMenuSub>
                            </CollapsibleContent>
                          </SidebarMenuItem>
                        </Collapsible>
                      )
                    }

                    return (
                      <SidebarMenuItem key={test.id}>
                        <SidebarMenuButton 
                          isActive={selectedTest === test.id}
                          onClick={() => handleSelection(() => test.id && setSelectedTest(test.id))}
                        >
                          {createElement(Icon)}
                          <span>{test.name}</span>
                        </SidebarMenuButton>
                      </SidebarMenuItem>
                    )
                  })}
                </SidebarMenu>
              </SidebarGroupContent>
            </CollapsibleContent>
          </SidebarGroup>
        </Collapsible>

        <Collapsible defaultOpen className="group/collapsible">
          <SidebarGroup>
            <SidebarGroupLabel asChild>
              <CollapsibleTrigger>
                <History className="mr-2 h-4 w-4" />
                Runs
                <ChevronDown className="ml-auto transition-transform group-data-[state=open]/collapsible:rotate-180" />
              </CollapsibleTrigger>
            </SidebarGroupLabel>
            <CollapsibleContent>
              <SidebarGroupContent>
                <SidebarMenu>
                  {sortedRuns.map((run) => (
                    <SidebarMenuItem key={run.id}>
                      <SidebarMenuButton 
                        isActive={selectedRunId === run.id}
                        onClick={() => handleSelection(() => setSelectedRunId(run.id))}
                        className="h-auto py-2 flex flex-col items-start"
                      >
                        <span className="font-medium">Run {run.id}</span>
                        <span className="text-xs text-muted-foreground">{new Date(run.createdAt).toLocaleDateString()}</span>
                      </SidebarMenuButton>
                    </SidebarMenuItem>
                  ))}
                </SidebarMenu>
              </SidebarGroupContent>
            </CollapsibleContent>
          </SidebarGroup>
        </Collapsible>

        <Collapsible defaultOpen className="group/collapsible">
          <SidebarGroup>
            <SidebarGroupLabel asChild>
              <CollapsibleTrigger>
                <Server className="mr-2 h-4 w-4" />
                Environment
                <ChevronDown className="ml-auto transition-transform group-data-[state=open]/collapsible:rotate-180" />
              </CollapsibleTrigger>
            </SidebarGroupLabel>
            <CollapsibleContent>
              <SidebarGroupContent>
                <SidebarMenu>
                  {environments.map((env) => {
                     const Icon = getIcon(env.icon)
                     return (
                      <SidebarMenuItem key={env.name}>
                        <SidebarMenuButton 
                          isActive={selectedEnvironmentName === env.name}
                          onClick={() => handleSelection(() => setSelectedEnvironment(env.name))}
                        >
                          <Icon />
                          <span>{env.displayName}</span>
                        </SidebarMenuButton>
                      </SidebarMenuItem>
                     )
                  })}
                </SidebarMenu>
              </SidebarGroupContent>
            </CollapsibleContent>
          </SidebarGroup>
        </Collapsible>
      </SidebarContent>
      
      {selectedEnvironment && (
        <SidebarFooter className="border-t p-4">
          <div className="text-sm font-medium flex items-center gap-2">
            {(() => {
              const Icon = getIcon(selectedEnvironment.icon)
              return <Icon className="h-4 w-4" />
            })()}
            {selectedEnvironment.displayName}
          </div>
          {selectedEnvironment.spec && (
            <div className="text-xs text-muted-foreground space-y-0.5">
              <div className="line-clamp-1">{selectedEnvironment.spec.split('\n').filter(l => l.trim())[0]}</div>
              {(() => {
                 const appLine = selectedEnvironment.spec.split('\n').find(l => l.trim().startsWith('APP:'))
                 return appLine ? <div className="line-clamp-1">{appLine}</div> : null
              })()}
            </div>
          )}
        </SidebarFooter>
      )}
    </Sidebar>
  )
}
