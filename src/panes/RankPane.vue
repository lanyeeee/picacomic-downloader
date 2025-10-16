<script setup lang="ts">
import { ref, watch } from 'vue'
import { commands, RankType } from '../bindings.ts'
import ComicCard from '../components/ComicCard.vue'
import { useStore } from '../store.ts'

const store = useStore()

const selectedRankType = ref<RankType>('Day')

watch(
  selectedRankType,
  () => {
    store.getRankResult = undefined
    getRank()
  },
  { immediate: true },
)

async function getRank() {
  const result = await commands.getRank(selectedRankType.value)
  if (result.status === 'error') {
    console.error(result.error)
    return
  }

  store.getRankResult = result.data
}
</script>

<template>
  <div class="h-full flex flex-col">
    <n-tabs class="my-2" v-model:value="selectedRankType" type="segment" size="small">
      <n-tab :name="'Day'" tab="过去24小时" />
      <n-tab :name="'Week'" tab="过去7天" />
      <n-tab :name="'Month'" tab="过去30天" />
    </n-tabs>
    <div v-if="store.getRankResult !== undefined" class="flex flex-col gap-row-2 overflow-auto box-border px-2 mb-2">
      <ComicCard
        v-for="comicInRank in store.getRankResult"
        :key="comicInRank.id"
        :comic-id="comicInRank.id"
        :comic-title="comicInRank.title"
        :comic-author="comicInRank.author"
        :comic-categories="comicInRank.categories"
        :comic-downloaded="comicInRank.is_downloaded"
        :comic-download-dir="comicInRank.comic_download_dir"
        :thumb="comicInRank.thumb" />
    </div>
  </div>
</template>

<style scoped>
:deep(.n-tabs-tab__label) {
  @apply h-5;
}

:deep(.n-tabs-rail) {
  @apply p-0;
}
</style>