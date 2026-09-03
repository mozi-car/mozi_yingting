import i18next from 'i18next'
import I18NextVue from 'i18next-vue'
import type { App } from 'vue'
import bundledEnglish from '../../../resources/locales/en/translation.json'
import bundledChinese from '../../../resources/locales/zh/translation.json'

type Translation = Record<string, any>

// Keep a renderer-side copy as a fallback.  The Tauri window can be created
// before the Node sidecar has finished starting; initializing i18next with an
// empty object in that case makes every component render the raw key forever.
const bundledTranslations: Record<string, Translation> = {
  en: bundledEnglish,
  zh: bundledChinese
}

// 翻译资源缓存
const resources: Record<string, { translation: Translation }> = {}

function isTranslation(value: unknown): value is Translation {
  return !!value && typeof value === 'object' && !Array.isArray(value) && Object.keys(value).length > 0
}

// 加载指定语言的翻译。IPC 失败时使用打包进 renderer 的资源，避免显示 home.home 等原始 key。
async function loadLanguage(lang: string): Promise<Translation> {
  try {
    const allTranslations = await window.electron.ipcRenderer.invoke('get-all-translations', lang)
    if (isTranslation(allTranslations)) return allTranslations
    console.warn(`[i18n] empty translations returned for ${lang}; using bundled resources`)
  } catch (error) {
    console.error(`Failed to load translations for ${lang}; using bundled resources:`, error)
  }

  return bundledTranslations[lang] ?? bundledTranslations.en
}

// 初始化 i18next - 只加载当前语言
export const initRendererI18n = async (lng: string = 'en') => {
  // check lng exists in supported languages
  const supportedLanguages = await getSupportedLanguages()
  if (!supportedLanguages.some((l) => l.code === lng)) {
    lng = 'en'
  }

  // 始终准备英文 fallback；当前语言优先从 sidecar 加载，失败时使用 bundled 资源。
  resources.en = { translation: bundledTranslations.en }
  const translations = await loadLanguage(lng)

  // 构建正确的 resources 结构: { lng: { translation: {...} } }
  resources[lng] = { translation: translations }

  await i18next.init({
    lng,
    fallbackLng: 'en',
    resources,
    ns: ['translation'],
    defaultNS: 'translation',
    debug: false,
    interpolation: {
      escapeValue: false
    }
  })
  return i18next
}

// 切换语言（按需加载新语言）
export const reloadRendererTranslations = async (lng: string) => {
  // 检查该语言是否已加载
  if (!resources[lng]) {
    // 加载新语言的翻译（IPC 不可用时由 loadLanguage 使用 bundled 资源）
    const translations = await loadLanguage(lng)
    resources[lng] = { translation: translations }

    // 添加到 i18next
    i18next.addResourceBundle(lng, 'translation', translations, true, true)
  }

  // 切换到新语言
  await i18next.changeLanguage(lng)

  // 保存语言偏好（主进程 conf + renderer 本地 localStorage，重启读取走后者）
  try {
    await window.electron.ipcRenderer.invoke('set-language', lng)
  } catch (error) {
    console.error('Failed to save language preference:', error)
  }
  try {
    window.store?.set?.('language', lng)
  } catch (error) {
    console.error('Failed to save language locally:', error)
  }
}

// 获取支持的语言列表
export const getSupportedLanguages = async () => {
  try {
    return await window.electron.ipcRenderer.invoke('get-supported-languages')
  } catch (error) {
    console.error('Failed to get supported languages:', error)
    return [{ code: 'en', name: 'English', nativeName: 'English' }]
  }
}

// Vue 插件
export const i18nPlugin = {
  install: (app: App) => {
    app.use(I18NextVue, { i18next })
  }
}
