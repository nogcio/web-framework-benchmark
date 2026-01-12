import { useEffect, useRef, useState } from 'react'
import axios from 'axios'
import { Area, AreaChart, CartesianGrid, XAxis, YAxis } from 'recharts'
import {
  Card,
  CardContent,
} from './ui/card'
import {
  type ChartConfig,
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
  ChartLegend,
  ChartLegendContent,
} from './ui/chart'
import { useAppStore } from '../store/useAppStore'
import type { TestCaseRaw } from '../types'
import { Skeleton } from './ui/skeleton'

interface Props {
  benchmarkName: string
}

const chartConfig = {
  rps: {
    label: "RPS",
    color: "#22c55e", // Green
  },
  memory: {
    label: "Memory (MB)",
    color: "#a855f7", // Purple
  },
  cpu: {
    label: "CPU (%)",
    color: "#3b82f6", // Blue
  },
  latency: {
    label: "Latency (ms)",
    color: "#f97316", // Orange
  }
} satisfies ChartConfig

export function BenchmarkCharts({ benchmarkName }: Props) {
  const [data, setData] = useState<TestCaseRaw[]>([])
  const [loading, setLoading] = useState(true)
  const abortControllerRef = useRef<AbortController | null>(null)
  
  const selectedRunId = useAppStore(s => s.selectedRunId)
  const selectedEnvironment = useAppStore(s => s.selectedEnvironment)
  const selectedTest = useAppStore(s => s.selectedTest)

  useEffect(() => {
    if (!selectedRunId || !selectedEnvironment || !selectedTest) {
      setLoading(false)
      return
    }

    // Reset state immediately for new benchmark
    setData([])
    setLoading(true)

    // Abort previous request if active
    if (abortControllerRef.current) {
      abortControllerRef.current.abort()
    }

    const controller = new AbortController()
    abortControllerRef.current = controller

    // Debounce to handle rapid hover/scrubbing and StrictMode double-mounts
    const timeoutId = setTimeout(() => {
      axios.get<TestCaseRaw[]>(
        `/api/runs/${selectedRunId}/environments/${selectedEnvironment}/tests/${selectedTest}/frameworks/${benchmarkName}/raw`,
        { signal: controller.signal }
      )
        .then(res => {
          if (!controller.signal.aborted) {
            setData(res.data)
            setLoading(false)
          }
        })
        .catch(err => {
          if (!axios.isCancel(err)) {
            if (!controller.signal.aborted) {
               setLoading(false)
            }
          }
        })
    }, 100)

    return () => {
      clearTimeout(timeoutId)
      controller.abort()
    }
  }, [benchmarkName, selectedRunId, selectedEnvironment, selectedTest])

  // Process data for charts
  const chartData = data.map((d) => ({
    time: d.elapsedSecs,
    rps: Math.round(d.requestsPerSec),
    memory: parseFloat((d.memoryUsageBytes / (1024 * 1024)).toFixed(1)), // MB
    cpu: parseFloat(d.cpuUsagePercent.toFixed(2)),
    latency: parseFloat(((d.latencyMean || 0) / 1000).toFixed(2)), // ms (assuming micros)
  }))

  const showChart = !loading && data.length > 0;

  return (
    <div className="grid grid-cols-1 gap-1 p-0 border-t">
      {/* RPS & Latency Chart */}
      <Card className="shadow-none border-none bg-muted/30 rounded-none">
        <CardContent className="px-4 py-0">
          {loading ? (
             <Skeleton className="h-[140px] w-full" />
          ) : showChart ? (
          <ChartContainer config={chartConfig} className="h-[140px] w-full">
            <AreaChart data={chartData} margin={{ left: 0, right: 0, top: 0, bottom: 0 }}>
              <CartesianGrid vertical={false} strokeDasharray="3 3"/>
              <XAxis
                dataKey="time"
                tickLine={false}
                axisLine={false}
                tickMargin={4}
                height={20}
                tickFormatter={(value) => `${value}s`}
                tick={{fontSize: 10}}
              />
              <YAxis yAxisId="left" orientation="left" tickLine={false} axisLine={false} tickFormatter={(value) => `${value}`} width={45} tick={{fontSize: 10}} />
              <YAxis yAxisId="right" orientation="right" tickLine={false} axisLine={false} tickFormatter={(value) => `${value}ms`} width={45} tick={{fontSize: 10}} />
              <ChartTooltip content={<ChartTooltipContent />} />
              <ChartLegend content={<ChartLegendContent className="flex-col items-start gap-1 pt-0 pb-0" />} layout="vertical" verticalAlign="middle" align="right" />
              <Area
                isAnimationActive={false}
                yAxisId="left"
                type="monotone"
                dataKey="rps"
                name="RPS"
                stroke={chartConfig.rps.color}
                fill={chartConfig.rps.color}
                fillOpacity={0.2}
                strokeWidth={2}
              />
              <Area
                isAnimationActive={false}
                yAxisId="right"
                type="monotone"
                dataKey="latency"
                name="Latency"
                stroke={chartConfig.latency.color}
                fill={chartConfig.latency.color}
                fillOpacity={0.1}
                strokeWidth={2}
              />
            </AreaChart>
          </ChartContainer>
          ) : (
            <div className="h-[120px] flex items-center justify-center text-muted-foreground text-sm">
              No data available
            </div>
          )}
        </CardContent>
      </Card>

      {/* CPU & Memory Chart */}
      <Card className="shadow-none border-none bg-muted/30 rounded-none">
        <CardContent className="px-4 py-0">
          {loading ? (
             <Skeleton className="h-[140px] w-full" />
          ) : showChart ? (
          <ChartContainer config={chartConfig} className="h-[140px] w-full">
            <AreaChart data={chartData} margin={{ left: 0, right: 0, top: 0, bottom: 0 }}>
              <CartesianGrid vertical={false} strokeDasharray="3 3"/>
              <XAxis dataKey="time" tickLine={false} axisLine={false} tickMargin={4} height={20} tickFormatter={(v) => `${v}s`} tick={{fontSize: 10}}/>
              <YAxis yAxisId="left" orientation="left" tickLine={false} axisLine={false} tickFormatter={(value) => `${value}%`} width={45} tick={{fontSize: 10}} />
              <YAxis yAxisId="right" orientation="right" tickLine={false} axisLine={false} tickFormatter={(value) => `${value}MB`} width={45} tick={{fontSize: 10}} />
              <ChartTooltip content={<ChartTooltipContent />} />
              <ChartLegend content={<ChartLegendContent className="flex-col items-start gap-1 pt-0 pb-0" />} layout="vertical" verticalAlign="middle" align="right" />
              <Area
                isAnimationActive={false}
                yAxisId="left"
                type="monotone"
                dataKey="cpu"
                name="CPU"
                stroke={chartConfig.cpu.color}
                fill={chartConfig.cpu.color}
                fillOpacity={0.2}
                strokeWidth={2}
              />
              <Area
                isAnimationActive={false}
                yAxisId="right"
                type="monotone"
                dataKey="memory"
                name="Memory"
                stroke={chartConfig.memory.color}
                fill={chartConfig.memory.color}
                fillOpacity={0.1}
                strokeWidth={2}
              />
            </AreaChart>
          </ChartContainer>
          ) : (
            <div className="h-[120px] flex items-center justify-center text-muted-foreground text-sm">
              No data available
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  )
}
