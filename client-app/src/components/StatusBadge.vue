<script setup lang="ts">
import { computed } from 'vue'
import { AlertTriangle, Check, Circle, Info } from '@lucide/vue'

type BadgeTone = 'success' | 'warning' | 'danger' | 'info' | 'neutral' | 'accent' | 'secondary'

const props = withDefaults(defineProps<{
  label: string
  tone?: BadgeTone
  compact?: boolean
  showIcon?: boolean
}>(), {
  tone: 'neutral',
  compact: false,
  showIcon: false,
})

const toneClass = computed(() => {
  const map: Record<BadgeTone, string> = {
    success: 'border-success/20 bg-success/10 text-success',
    warning: 'border-warning/20 bg-warning/10 text-warning',
    danger: 'border-danger/20 bg-danger/10 text-danger',
    info: 'border-info/20 bg-info-soft text-info',
    neutral: 'border-border bg-bg-secondary text-text-secondary',
    accent: 'border-accent/20 bg-accent-soft text-accent',
    secondary: 'border-secondary/25 bg-secondary-soft text-secondary',
  }
  return map[props.tone]
})

const iconComponent = computed(() => {
  if (props.tone === 'success' || props.tone === 'accent') return Check
  if (props.tone === 'warning' || props.tone === 'danger' || props.tone === 'secondary') return AlertTriangle
  if (props.tone === 'info') return Info
  return Circle
})
</script>

<template>
  <span
    class="inline-flex items-center gap-1 rounded-full border font-semibold leading-none"
    :class="[
      toneClass,
      compact ? 'px-1.5 py-0.5 text-[10px]' : 'px-2 py-1 text-[11px]',
    ]"
  >
    <component
      :is="iconComponent"
      v-if="showIcon"
      class="h-3 w-3"
      :stroke-width="2.4"
      aria-hidden="true"
    />
    {{ label }}
  </span>
</template>
