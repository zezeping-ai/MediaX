<script setup lang="ts">
import { useResizeObserver } from "@vueuse/core";
import { nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import type { ECharts, EChartsCoreOption } from "echarts/core";

const props = defineProps<{
  bars: number[];
  holdBars: number[];
  peakHold: number;
  compact?: boolean;
}>();

const chartEl = ref<HTMLDivElement | null>(null);
let chart: ECharts | null = null;
let echartsModule: (typeof import("echarts/core")) | null = null;
let chartsRegistered = false;
let rafHandle: number | null = null;

const CATEGORY_COUNT = 24;
const DB_MARKS = [0, -6, -12, -20, -40, -54] as const;
const FREQ_LABELS = new Map([
  [0, "32"],
  [5, "125"],
  [10, "500"],
  [15, "2k"],
  [20, "8k"],
  [23, "16k"],
]);
const CATEGORIES = Array.from({ length: CATEGORY_COUNT }, (_, index) => index);
const DB_AXIS_VALUES = DB_MARKS.map(normalizedFromDbfs);

function clamp01(value: number) {
  return Math.max(0, Math.min(1, Number.isFinite(value) ? value : 0));
}

function normalizedFromDbfs(db: number) {
  return clamp01((db + 54) / 54);
}

function axisDbLabel(value: number) {
  const matched = DB_MARKS.find((db) => Math.abs(normalizedFromDbfs(db) - value) <= 0.012);
  return matched === undefined ? "" : `${matched}`;
}

function buildOption(): EChartsCoreOption {
  const peakLine = clamp01(props.peakHold);
  const isCompact = Boolean(props.compact);
  return {
    animation: false,
    backgroundColor: "transparent",
    grid: {
      top: isCompact ? 1 : 2,
      right: 0,
      bottom: isCompact ? 10 : 12,
      left: 26,
      containLabel: false,
    },
    xAxis: {
      type: "category",
      data: CATEGORIES,
      boundaryGap: true,
      axisLine: {
        show: false,
      },
      axisTick: {
        show: false,
      },
      axisLabel: {
        interval: 0,
        color: "rgba(255,255,255,0.30)",
        fontSize: 9,
        margin: isCompact ? 4 : 6,
        formatter: (value: string) => FREQ_LABELS.get(Number(value)) ?? "",
      },
      splitLine: {
        show: true,
        lineStyle: {
          color: "rgba(255,255,255,0.08)",
          width: 1,
        },
      },
    },
    yAxis: {
      type: "value",
      min: 0,
      max: 1,
      interval: 0.2,
      axisLine: {
        show: false,
      },
      axisTick: {
        show: false,
      },
      axisLabel: {
        color: "rgba(255,255,255,0.24)",
        fontSize: 9,
        margin: isCompact ? 4 : 6,
        formatter: (value: number) => axisDbLabel(value),
      },
      splitLine: {
        show: true,
        lineStyle: {
          color: "rgba(255,255,255,0.08)",
          width: 1,
        },
      },
      minorTick: {
        show: false,
      },
      minorSplitLine: {
        show: false,
      },
    },
    series: [
      {
        type: "bar",
        data: buildBarData(props.bars),
        barMaxWidth: isCompact ? 9 : 10,
        barMinHeight: isCompact ? 5 : 6,
        itemStyle: {
          color: new echartsModule!.graphic.LinearGradient(0, 0, 0, 1, [
            { offset: 0, color: "rgba(255,255,255,0.92)" },
            { offset: 1, color: "rgba(255,255,255,0.18)" },
          ]),
          opacity: 0.92,
        },
        emphasis: {
          disabled: true,
        },
        markLine: {
          silent: true,
          symbol: "none",
          animation: false,
          lineStyle: {
            color: "rgba(255,255,255,0.28)",
            width: 1,
          },
          label: {
            show: false,
          },
          data: [{ yAxis: peakLine }],
        },
        z: 2,
      },
      {
        type: "scatter",
        data: buildHoldData(props.holdBars),
        symbol: "rect",
        symbolSize: isCompact ? [10, 2] : [11, 2],
        itemStyle: {
          color: "rgba(255,255,255,0.58)",
        },
        silent: true,
        emphasis: {
          disabled: true,
        },
        z: 3,
      },
      {
        type: "scatter",
        data: DB_AXIS_VALUES.map((value) => [0, value]),
        symbolSize: 0,
        silent: true,
        tooltip: {
          show: false,
        },
        z: 0,
      },
    ],
    tooltip: {
      show: false,
    },
  };
}

async function loadEcharts() {
  if (echartsModule) {
    return echartsModule;
  }
  const [core, charts, components, renderers] = await Promise.all([
    import("echarts/core"),
    import("echarts/charts"),
    import("echarts/components"),
    import("echarts/renderers"),
  ]);
  echartsModule = core;
  if (!chartsRegistered) {
    echartsModule.use([
      charts.BarChart,
      charts.ScatterChart,
      components.GridComponent,
      renderers.CanvasRenderer,
    ]);
    chartsRegistered = true;
  }
  return echartsModule;
}

async function ensureChart() {
  if (!chartEl.value) {
    return;
  }
  const echarts = await loadEcharts();
  if (!chart) {
    chart = echarts.init(chartEl.value, undefined, { renderer: "canvas" });
  }
  chart.setOption(buildOption(), true);
}

function scheduleChartDataUpdate() {
  if (rafHandle !== null) {
    return;
  }
  rafHandle = window.requestAnimationFrame(() => {
    rafHandle = null;
    if (!chart) {
      return;
    }
    chart.setOption({
      series: [
        {
          data: buildBarData(props.bars),
          markLine: {
            data: [{ yAxis: clamp01(props.peakHold) }],
          },
        },
        {
          data: buildHoldData(props.holdBars),
        },
      ],
    });
  });
}

watch(
  () => [props.compact, props.bars, props.holdBars, props.peakHold],
  () => {
    if (!chart) {
      void ensureChart();
      return;
    }
    scheduleChartDataUpdate();
  },
  { deep: true },
);

useResizeObserver(chartEl, () => {
  chart?.resize();
});

onMounted(async () => {
  await nextTick();
  await ensureChart();
});

onBeforeUnmount(() => {
  if (rafHandle !== null) {
    window.cancelAnimationFrame(rafHandle);
    rafHandle = null;
  }
  chart?.dispose();
  chart = null;
});

function buildBarData(values: number[]) {
  return Array.from({ length: CATEGORY_COUNT }, (_, index) => clamp01(values[index] ?? 0));
}

function buildHoldData(values: number[]) {
  return Array.from({ length: CATEGORY_COUNT }, (_, index) => [index, clamp01(values[index] ?? 0)]);
}
</script>

<template>
  <div ref="chartEl" :class="props.compact ? 'h-[6.75rem] w-full' : 'h-[9.5rem] w-full'" />
</template>
