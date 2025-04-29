<script setup lang="ts">
import ComicCard from '../components/ComicCard.vue'
import { computed, ref, watch } from 'vue'
import { ComicInfo } from '../types.ts'
import { ComicInFavoriteRespData, commands, Pagination, Sort } from '../bindings.ts'
import { useStore } from '../store.ts'

const store = useStore()

const comicInFavoritePagination = ref<Pagination<ComicInFavoriteRespData>>()
const sortSelected = ref<Sort>('TimeNewest') // TODO: 添加一个选择器来控制这个值

const comicInfoPagination = computed<Pagination<ComicInfo> | undefined>(() => {
  const pagination = comicInFavoritePagination.value
  if (pagination === undefined) {
    return undefined
  }
  return {
    ...pagination,
    docs: pagination.docs.map(({ _id, title, author, categories, thumb }) => ({
      _id,
      title,
      author,
      categories,
      thumb,
    })),
  }
})

async function getFavorite(sort: Sort, page: number) {
  const result = await commands.getFavoriteComics(sort, page)
  if (result.status === 'error') {
    console.error(result.error)
    return
  }
  comicInFavoritePagination.value = result.data
}

watch(
  () => store.currentTabName,
  async () => {
    if (comicInFavoritePagination.value !== undefined || store.currentTabName !== 'favorite') {
      return
    }
    await getFavorite(sortSelected.value, 1)
  },
  { immediate: true },
)
</script>

<template>
  <div v-if="comicInfoPagination !== undefined" class="h-full flex flex-col gap-2">
    <div class="flex flex-col gap-row-2 overflow-auto box-border px-2 pt-2">
      <comic-card v-for="comicInfo in comicInfoPagination.docs" :key="comicInfo._id" :comic-info="comicInfo" />
    </div>
    <n-pagination
      class="box-border p-2 pt-0 mt-auto"
      :page-count="comicInfoPagination.pages"
      :page="comicInfoPagination.page"
      @update:page="getFavorite(sortSelected, $event)" />
  </div>
</template>
