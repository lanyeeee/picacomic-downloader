<script setup lang="ts">

import ComicCard from "./ComicCard.vue";
import {computed, ref, watch} from "vue";
import {ComicInfo} from "../types.ts";
import {ComicInFavoriteRespData, commands, Pagination, Sort} from "../bindings.ts";
import {useNotification} from "naive-ui";

const notification = useNotification();

const props = defineProps<{
  searchById: (comicId: string) => void;
  currentTabName: "search" | "favorite" | "episode";
}>();

const comicInFavoritePagination = ref<Pagination<ComicInFavoriteRespData>>();
const sortSelected = ref<Sort>("TimeNewest");// TODO: 添加一个选择器来控制这个值

const comicInfoPagination = computed<Pagination<ComicInfo> | undefined>(() => {
  const pagination = comicInFavoritePagination.value;
  if (pagination === undefined) {
    return undefined;
  }
  return {
    ...pagination,
    docs: pagination.docs.map(({_id, title, author, categories, thumb}) => ({_id, title, author, categories, thumb,})),
  };
});

async function getFavorite(sort: Sort, page: number) {
  const result = await commands.getFavoriteComics(sort, page);
  if (result.status === "error") {
    notification.error({title: "获取收藏失败", description: result.error});
    return;
  }
  comicInFavoritePagination.value = result.data;
}

watch(() => props.currentTabName, async () => {
  if (comicInFavoritePagination.value !== undefined || props.currentTabName !== "favorite") {
    return;
  }
  await getFavorite(sortSelected.value, 1);
}, {immediate: true});

</script>

<template>
  <div class="h-full flex flex-col">
    <div v-if="comicInfoPagination!==undefined" class="flex flex-col gap-row-1 overflow-auto p-2">
      <div class="flex flex-col gap-row-2 overflow-auto">
        <comic-card v-for="comicInfo in comicInfoPagination.docs"
                    :key="comicInfo._id"
                    :comic-info="comicInfo"
                    :onClickItem="searchById"/>
      </div>
      <n-pagination :total="comicInfoPagination.total"
                    :page-count="comicInfoPagination.pages"
                    :page="comicInfoPagination.page"
                    @update:page="getFavorite(sortSelected, $event)"/>
    </div>
  </div>
</template>
