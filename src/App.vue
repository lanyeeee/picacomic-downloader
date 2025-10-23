<script setup lang="ts">
// This starter template is using Vue 3 <script setup> SFCs
// Check out https://vuejs.org/api/sfc-script-setup.html#script-setup
import AppContent from './AppContent.vue'
import { GlobalThemeOverrides } from 'naive-ui'

const themeOverrides: GlobalThemeOverrides = {
  common: {
    primaryColor: '#DB547C',
    primaryColorHover: '#E87D9A',
    primaryColorPressed: '#B53C64',
    primaryColorSuppl: '#E87D9A',
    borderRadius: '4px',
    borderRadiusSmall: '3px',
    heightMedium: '32px',
  },
  Button: {
    paddingSmall: '0 8px',
    paddingMedium: '0 12px',
  },
  Radio: {
    buttonColorActive: '#DB547C',
    buttonTextColorActive: '#FFF',
  },
  Dropdown: {
    borderRadius: '5px',
    padding: '6px 2px',
    optionColorHover: '#DB547C',
    optionTextColorHover: '#FFF',
    optionHeightMedium: '28px',
  },
  Tabs: {
    tabTextColorActiveSegment: '#DB547C',
    tabTextColorHoverSegment: '#E87D9A',
    tabColorSegment: '#FFDAE9',
    colorSegment: '#FFFFFFFF',
  },
}
</script>

<template>
  <n-config-provider :theme-overrides="themeOverrides">
    <n-modal-provider>
      <n-notification-provider placement="bottom-right" :max="3">
        <n-message-provider>
          <app-content />
        </n-message-provider>
      </n-notification-provider>
    </n-modal-provider>
  </n-config-provider>
</template>
