<script setup lang="ts">
import ComicCard from '../components/ComicCard.vue'
import { computed, ref, watch } from 'vue'
import { ComicInfo, CurrentTabName } from '../types.ts'
import { ComicInFavoriteRespData, commands, Pagination, Sort } from '../bindings.ts'
import { useNotification } from 'naive-ui'

const notification = useNotification()

const props = defineProps<{
  searchById: (comicId: string) => void
  currentTabName: CurrentTabName
}>()

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
    notification.error({ title: '获取收藏失败', description: result.error })
    return
  }
  comicInFavoritePagination.value = result.data
}

watch(
  () => props.currentTabName,
  async () => {
    if (comicInFavoritePagination.value !== undefined || props.currentTabName !== 'favorite') {
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
      <comic-card
        v-for="comicInfo in comicInfoPagination.docs"
        :key="comicInfo._id"
        :comic-info="comicInfo"
        :onClickItem="searchById" />
    </div>
    <n-pagination
      class="box-border p-2 pt-0 mt-auto"
      :page-count="comicInfoPagination.pages"
      :page="comicInfoPagination.page"
      @update:page="getFavorite(sortSelected, $event)" />
  </div>
</template>
