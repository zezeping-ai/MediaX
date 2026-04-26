<script setup lang="ts">
import { useResizeObserver } from "@vueuse/core";
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import type { ECharts, EChartsOption } from "echarts/core";

const props = defineProps<{
  bars: number[];
  holdBars: number[];
  peakHold: number;
}>();

const chartEl = ref<HTMLDivElement | null>(null);
let chart: ECharts | null = null;
let echartsModule: (typeof import("echarts/core")) | null = null;
let chartsRegistered = false;

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

const categories = computed(() => Array.from({ length: CATEGORY_COUNT }, (_, index) => index));
const barData = computed(() =>
  Array.from({ length: CATEGORY_COUNT }, (_, index) => clamp01(props.bars[index] ?? 0)),
);
const holdData = computed(() =>
  Array.from({ length: CATEGORY_COUNT }, (_, index) => clamp01(props.holdBars[index] ?? 0)),
);
const peakHoldValue = computed(() => clamp01(props.peakHold));
const dbAxisValues = computed(() => DB_MARKS.map(normalizedFromDbfs));

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

function buildOption(): EChartsOption {
  const peakLine = peakHoldValue.value;
  return {
    animation: false,
    backgroundColor: "transparent",
    grid: {
      top: 6,
      right: 0,
      bottom: 18,
      left: 26,
      containLabel: false,
    },
    xAxis: {
      type: "category",
      data: categories.value,
      boundaryGap: true,
      axisLine: {
        lineStyle: {
          color: "rgba(255,255,255,0.12)",
        },
      },
      axisTick: {
        show: false,
      },
      axisLabel: {
        interval: 0,
        color: "rgba(255,255,255,0.30)",
        fontSize: 9,
        margin: 10,
        formatter: (value: string) => FREQ_LABELS.get(Number(value)) ?? "",
      },
      splitLine: {
        show: false,
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
        margin: 8,
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
        data: barData.value,
        barMaxWidth: 7,
        barMinHeight: 3,
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
        data: holdData.value.map((value, index) => [index, value]),
        symbol: "rect",
        symbolSize: [8, 2],
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
        data: dbAxisValues.value.map((value) => [0, value]),
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

watch([barData, holdData, peakHoldValue], () => {
  void ensureChart();
}, { deep: true });

useResizeObserver(chartEl, () => {
  chart?.resize();
});

onMounted(async () => {
  await nextTick();
  await ensureChart();
});

onBeforeUnmount(() => {
  chart?.dispose();
  chart = null;
});
</script>

<template>
  <div ref="chartEl" class="h-[5.5rem] w-full" />
</template>
