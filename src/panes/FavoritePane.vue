<script setup lang="ts">
import ComicCard from '../components/ComicCard.vue'
import { ref, watch } from 'vue'
import { ComicInFavoriteRespData, commands, Pagination, Sort } from '../bindings.ts'
import { useStore } from '../store.ts'

const store = useStore()

const comicInFavoritePagination = ref<Pagination<ComicInFavoriteRespData>>()
const sortSelected = ref<Sort>('TimeNewest') // TODO: 添加一个选择器来控制这个值

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
  <div v-if="comicInFavoritePagination !== undefined" class="h-full flex flex-col gap-2">
    <div class="flex flex-col gap-row-2 overflow-auto box-border px-2 pt-2">
      <comic-card
        v-for="{ _id, title, author, categories, thumb } in comicInFavoritePagination.docs"
        :key="_id"
        :comic-id="_id"
        :comic-title="title"
        :comic-author="author"
        :comic-categories="categories"
        :thumb="thumb" />
    </div>
    <n-pagination
      class="box-border p-2 pt-0 mt-auto"
      :page-count="comicInFavoritePagination.pages"
      :page="comicInFavoritePagination.page"
      @update:page="getFavorite(sortSelected, $event)" />
  </div>
</template>
