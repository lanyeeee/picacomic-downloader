<script setup lang="ts">
import ComicCard from '../components/ComicCard.vue'
import { ref, watch } from 'vue'
import { commands, GetFavoriteSort } from '../bindings.ts'
import { useStore } from '../store.ts'
import { SelectProps } from 'naive-ui'

const store = useStore()

const sortOptions: SelectProps['options'] = [
  { label: '新到旧', value: 'TimeNewest' },
  { label: '旧到新', value: 'TimeOldest' },
]

const sortSelected = ref<GetFavoriteSort>('TimeNewest') // TODO: 添加一个选择器来控制这个值

async function getFavorite(sort: GetFavoriteSort, page: number) {
  console.log('getFavorite', sort, page)
  const result = await commands.getFavorite(sort, page)
  if (result.status === 'error') {
    console.error(result.error)
    return
  }
  store.getFavoriteResult = result.data
}

watch(
  () => store.currentTabName,
  async () => {
    if (store.getFavoriteResult !== undefined || store.currentTabName !== 'favorite') {
      return
    }
    await getFavorite(sortSelected.value, 1)
  },
  { immediate: true },
)
</script>

<template>
  <div v-if="store.getFavoriteResult !== undefined" class="h-full flex flex-col gap-2">
    <n-input-group class="box-border px-2 pt-2">
      <n-input-group-label size="small">排序方式</n-input-group-label>
      <n-select
        class="w-25"
        v-model:value="sortSelected"
        :options="sortOptions"
        :show-checkmark="false"
        size="small"
        @update-value="getFavorite($event, 1)" />
    </n-input-group>
    <div class="flex flex-col gap-row-2 overflow-auto box-border px-2">
      <comic-card
        v-for="{ id, title, author, categories, thumb, isDownloaded } in store.getFavoriteResult.docs"
        :key="id"
        :comic-id="id"
        :comic-title="title"
        :comic-author="author"
        :comic-categories="categories"
        :comic-downloaded="isDownloaded"
        :thumb="thumb" />
    </div>
    <n-pagination
      class="box-border p-2 pt-0 mt-auto"
      :page-count="store.getFavoriteResult.pages"
      :page="store.getFavoriteResult.page"
      @update:page="getFavorite(sortSelected, $event)" />
  </div>
</template>
