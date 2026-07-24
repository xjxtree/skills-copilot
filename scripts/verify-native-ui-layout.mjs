#!/usr/bin/env node

import { readFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = dirname(dirname(fileURLToPath(import.meta.url)));

const files = {
  app: await read("apps/macos/Sources/SkillsCopilot/App/SkillsCopilotApp.swift"),
  appTheme: await read("apps/macos/Sources/SkillsCopilot/Models/AppTheme.swift"),
  appThemePlatform: await read("apps/macos/Sources/SkillsCopilot/App/AppThemePlatform.swift"),
  mainWindowCoordinator: await read("apps/macos/Sources/SkillsCopilot/App/MainWindowCoordinator.swift"),
  mainWindowModel: await read("apps/macos/Sources/SkillsCopilot/Models/MainWindowModel.swift"),
  content: await read("apps/macos/Sources/SkillsCopilot/Views/ContentView.swift"),
  advancedWorkspace: await read(
    "apps/macos/Sources/SkillsCopilot/Views/AdvancedWorkspaceView.swift",
  ),
  projectOverview: await read(
    "apps/macos/Sources/SkillsCopilot/Views/ProjectOverviewView.swift",
  ),
  projectOverviewPreview: await read(
    "apps/macos/Sources/SkillsCopilot/Views/ProjectOverviewPreviewSheet.swift",
  ),
  projectOverviewModel: await read(
    "apps/macos/Sources/SkillsCopilot/Models/ProjectOverviewPresentation.swift",
  ),
  skillsWorkspace: await read(
    "apps/macos/Sources/SkillsCopilot/Views/SkillsWorkspaceView.swift",
  ),
  skillsWorkspaceList: await read(
    "apps/macos/Sources/SkillsCopilot/Views/SkillsWorkspaceListView.swift",
  ),
  skillAggregateDetail: await read(
    "apps/macos/Sources/SkillsCopilot/Views/SkillAggregateDetailView.swift",
  ),
  skillsWorkspaceModel: await read(
    "apps/macos/Sources/SkillsCopilot/Models/SkillsWorkspaceListPresentation.swift",
  ),
  sessionsWorkspaceList: await read(
    "apps/macos/Sources/SkillsCopilot/Views/SessionsWorkspaceListView.swift",
  ),
  sessionsWorkspaceModel: await read(
    "apps/macos/Sources/SkillsCopilot/Models/SessionWorkspaceListPresentation.swift",
  ),
  sessionWorkspaceStore: await read(
    "apps/macos/Sources/SkillsCopilot/Stores/SessionWorkspaceStore.swift",
  ),
  skillManagerEntryContext: await read(
    "apps/macos/Sources/SkillsCopilot/Models/SkillManagerEntryContext.swift",
  ),
  skillWorkspaceStore: await read(
    "apps/macos/Sources/SkillsCopilot/Stores/SkillWorkspaceStore.swift",
  ),
  sessionWorkspaceDetail: await read(
    "apps/macos/Sources/SkillsCopilot/Views/SessionWorkspaceDetailView.swift",
  ),
  sessionWorkspaceDetailModel: await read(
    "apps/macos/Sources/SkillsCopilot/Models/SessionWorkspaceDetailPresentation.swift",
  ),
  detailPrimitives: await read("apps/macos/Sources/SkillsCopilot/Views/DetailPresentationPrimitives.swift"),
  providerObservabilitySettings: await read("apps/macos/Sources/SkillsCopilot/Views/ProviderObservabilitySettingsPanel.swift"),
  legacyPrivateContentCard: await read("apps/macos/Sources/SkillsCopilot/Views/LegacyPrivateContentCleanupCard.swift"),
  legacyPrivateContentBanner: await read("apps/macos/Sources/SkillsCopilot/Views/LegacyPrivateContentGlobalBanner.swift"),
  settingsNavigation: await read("apps/macos/Sources/SkillsCopilot/Views/SettingsNavigation.swift"),
  skillManager: await read("apps/macos/Sources/SkillsCopilot/Views/SkillManagerPanel.swift"),
  workflowSheet: await read("apps/macos/Sources/SkillsCopilot/Views/WorkflowSheetChrome.swift"),
  skillManagerModel: await read("apps/macos/Sources/SkillsCopilot/Models/SkillManager.swift"),
  batchSkillOperation: await read("apps/macos/Sources/SkillsCopilot/Views/BatchSkillOperationSheet.swift"),
  markdownRender: await read("apps/macos/Sources/SkillsCopilot/Models/MarkdownRenderDocument.swift"),
  markdownTableDisplay: await read("apps/macos/Sources/SkillsCopilot/Models/MarkdownTableDisplayModel.swift"),
  agentConfigWorkspace: await read("apps/macos/Sources/SkillsCopilot/Views/AgentConfigWorkspacePanel.swift"),
  configSnapshotPreview: await read(
    "apps/macos/Sources/SkillsCopilot/Views/ConfigSnapshotPreview.swift",
  ),
  agentIconProvider: await read("apps/macos/Sources/SkillsCopilot/Support/AgentIconProvider.swift"),
  formatter: await read("apps/macos/Sources/SkillsCopilot/Support/Formatters.swift"),
  uiStrings: await read("apps/macos/Sources/SkillsCopilot/Support/UIStrings.swift"),
  privacyPath: await read("apps/macos/Sources/SkillsCopilot/Views/PrivacyPathView.swift"),
  serviceClient: await read("apps/macos/Sources/SkillsCopilot/Services/ServiceClient.swift"),
  serviceClientTransport: await read("apps/macos/Sources/SkillsCopilot/Services/ServiceClientTransport.swift"),
  serviceProcessRunner: await read("apps/macos/Sources/SkillsCopilot/Services/ServiceProcessRunner.swift"),
  settings: await read("apps/macos/Sources/SkillsCopilot/Views/SettingsView.swift"),
  sidebar: await read("apps/macos/Sources/SkillsCopilot/Views/SidebarView.swift"),
  sidebarSelection: await read("apps/macos/Sources/SkillsCopilot/Models/SidebarSelection.swift"),
  uiOptimization: await read("apps/macos/Sources/SkillsCopilot/Models/UIOptimizationPresentation.swift"),
  confirmedMutationLane: await read("apps/macos/Sources/SkillsCopilot/Models/ConfirmedMutationLane.swift"),
  localSessionCache: await read("apps/macos/Sources/SkillsCopilot/Models/LocalSessionCache.swift"),
  listCompletenessControls: await read("apps/macos/Sources/SkillsCopilot/Views/ListCompletenessControls.swift"),
  store: await read("apps/macos/Sources/SkillsCopilot/Stores/SkillStore.swift"),
  storeProjectionHelpers: await read("apps/macos/Sources/SkillsCopilot/Stores/SkillStore+ProjectionHelpers.swift"),
  storeLegacyPrivacy: await read("apps/macos/Sources/SkillsCopilot/Stores/SkillStore+LegacyPrivacyCleanup.swift"),
  storeLocalSessionDetail: await read("apps/macos/Sources/SkillsCopilot/Stores/SkillStore+LocalSessionDetail.swift"),
  storePresentationModels: await read("apps/macos/Sources/SkillsCopilot/Stores/SkillStorePresentationModels.swift"),
  storeList: await read("apps/macos/Sources/SkillsCopilot/Stores/SkillListModel.swift"),
  storeDerivedState: await read("apps/macos/Sources/SkillsCopilot/Stores/SkillStoreDerivedState.swift"),
  storeWorkflow: await read("apps/macos/Sources/SkillsCopilot/Stores/SkillStoreWorkflowSelectors.swift"),
  taskCockpit: await read("apps/macos/Sources/SkillsCopilot/Views/TaskCockpitPanel.swift"),
  taskCockpitModel: await read("apps/macos/Sources/SkillsCopilot/Models/TaskCockpit.swift"),
  taskInput: await read("apps/macos/Sources/SkillsCopilot/Views/TaskInputTextEditor.swift"),
  taskInputModel: await read("apps/macos/Sources/SkillsCopilot/Models/TaskInputModel.swift"),
  nativePanelSurface: await read("apps/macos/Sources/SkillsCopilot/Views/NativePanelSurface.swift"),
  localizable: await read("apps/macos/Sources/SkillsCopilot/Resources/en.lproj/Localizable.strings"),
  localizableZh: await read("apps/macos/Sources/SkillsCopilot/Resources/zh-Hans.lproj/Localizable.strings"),
  serviceProtocol: await read("docs/service-protocol.md"),
  serviceStatusFixture: await read("fixtures/service-protocol/service.status.response.json"),
  serviceRust: await read("crates/service/src/lib.rs"),
  serviceLLM: await read("crates/service/src/service_llm.rs"),
  serviceLLMPromptHelpers: await read("crates/service/src/service_llm_prompt_helpers.rs"),
  serviceRustProtocol: await read("crates/service/src/protocol.rs"),
};
const retiredPresentationPaths = [
  "apps/macos/Sources/SkillsCopilot/Views/DetailView.swift",
  "apps/macos/Sources/SkillsCopilot/Views/AgentSessionDetailPanel.swift",
  "apps/macos/Sources/SkillsCopilot/Models/DetailSection.swift",
  "apps/macos/Sources/SkillsCopilot/Views/DetailOverviewSection.swift",
  "apps/macos/Sources/SkillsCopilot/Views/DetailHeaderOverviewSection.swift",
  "apps/macos/Sources/SkillsCopilot/Views/DetailFindingsHistorySection.swift",
  "apps/macos/Sources/SkillsCopilot/Stores/TaskCockpitHistoryStore.swift",
];
const presentRetiredPresentationPaths = (
  await Promise.all(
    retiredPresentationPaths.map(async (path) => [path, await exists(path)]),
  )
).filter(([, present]) => present);
files.detailSurface = [
  files.skillAggregateDetail,
  files.sessionWorkspaceDetail,
  files.detailPrimitives,
  files.providerObservabilitySettings,
  files.agentConfigWorkspace,
  files.configSnapshotPreview,
  files.taskCockpit,
].join("\n");
files.serviceIPC = [
  files.serviceClient,
  files.serviceClientTransport,
  files.serviceProcessRunner,
].join("\n");
files.storeSurface = [
  files.store,
  files.storeProjectionHelpers,
  files.storeLegacyPrivacy,
  files.storeDerivedState,
  files.storeWorkflow,
].join("\n");
files.serviceRustSurface = [
  files.serviceRust,
  files.serviceLLM,
  files.serviceLLMPromptHelpers,
  files.serviceRustProtocol,
].join("\n");

const runServiceBody = extractFunctionBody(files.serviceIPC, "runService");
const serviceRequestBody = extractServiceRequestBody(files.serviceIPC);
const supportedMethods = parseSupportedMethods(files.serviceRustSurface);
const statusFixtureMethods = parseStatusFixtureMethods(files.serviceStatusFixture);
const forbiddenProtocolMethods = supportedMethods.filter((method) => /^(ipc|sidecar|daemon|process|socket)\./.test(method));

const checks = [
  {
    label: "shared list completeness controls expose stable accessibility identifiers",
    text: files.listCompletenessControls,
    passed: /struct ListCompletenessBadge:[\s\S]*?list-completeness\.badge/.test(files.listCompletenessControls)
      && /struct ListCompletenessFooter:[\s\S]*?Text\(visibleSummary\)[\s\S]*?private var visibleSummary:[\s\S]*?UIStrings\.listCompletenessSummary\(/.test(files.listCompletenessControls)
      && /struct ListPagingActions:[\s\S]*?list-completeness\.load-more[\s\S]*?list-completeness\.load-all[\s\S]*?list-completeness\.cancel/.test(files.listCompletenessControls)
      && /struct ExpandableSummaryList<[\s\S]*?list-completeness\.show-all/.test(files.listCompletenessControls),
  },
  {
    label: "skill workspace exposes accepted catalog completeness and instance evidence",
    text: files.skillsWorkspaceList + "\n" + files.skillAggregateDetail,
    passed: /SkillsWorkspaceListPresentation\([\s\S]*?completeness:\s*workspace\.listCompleteness/.test(files.skillsWorkspaceList)
      && /ListCompletenessFooter\([\s\S]*?state:\s*presentation\.completeness/.test(files.skillsWorkspaceList)
      && /accessibilityIdentifierPrefix:\s*"skills-workspace"/.test(files.skillsWorkspaceList)
      && /SkillAggregateInstanceEvidenceRow/.test(files.skillAggregateDetail),
  },
  {
    label: "app window defines stable minimum size and user-selectable appearance",
    text: files.app + "\n" + files.appTheme + "\n" + files.appThemePlatform + "\n" + files.mainWindowCoordinator + "\n" + files.mainWindowModel,
    passed: /static let minimumWidth = 1349/.test(files.mainWindowModel)
      && /static let minimumHeight = 600/.test(files.mainWindowModel)
      && /\.frame\(minWidth:\s*CGFloat\(MainWindowModel\.minimumWidth\),\s*minHeight:\s*CGFloat\(MainWindowModel\.minimumHeight\)\)/.test(files.app)
      && /applicationDidFinishLaunching[\s\S]*?MainWindowCoordinator\.configureApplicationAppearance\(\)/.test(files.app)
      && /@AppStorage\(AppTheme\.storageKey\)[\s\S]*?appThemeRawValue/.test(files.app)
      && /let appTheme = AppTheme\.fromStorage\(appThemeRawValue\)/.test(files.app)
      && /ContentView\(\)[\s\S]*?\.preferredColorScheme\(appTheme\.colorScheme\)[\s\S]*?MainWindowConfigurator\(theme:\s*appTheme\)/.test(files.app)
      && /SettingsView\(\)[\s\S]*?\.preferredColorScheme\(appTheme\.colorScheme\)/.test(files.app)
      && /enum AppTheme:[\s\S]*?case system[\s\S]*?case light[\s\S]*?case dark[\s\S]*?static let defaultTheme = AppTheme\.system/.test(files.appTheme)
      && /extension AppTheme[\s\S]*?var colorScheme:[\s\S]*?case \.system:[\s\S]*?return nil[\s\S]*?case \.light:[\s\S]*?return \.light[\s\S]*?case \.dark:[\s\S]*?return \.dark/.test(files.appThemePlatform)
      && /extension AppTheme[\s\S]*?var nsAppearance:[\s\S]*?case \.system:[\s\S]*?return nil[\s\S]*?case \.light:[\s\S]*?NSAppearance\(named:\s*\.aqua\)[\s\S]*?case \.dark:[\s\S]*?NSAppearance\(named:\s*\.darkAqua\)/.test(files.appThemePlatform)
      && /static func configureApplicationAppearance\(_ theme: AppTheme = \.current,[\s\S]*?app\.appearance = theme\.nsAppearance/.test(files.mainWindowCoordinator)
      && /static func configureWindow\(_ window: NSWindow,\s*theme: AppTheme = \.current\)[\s\S]*?window\.appearance = theme\.nsAppearance[\s\S]*?window\.isMovableByWindowBackground = false/.test(files.mainWindowCoordinator),
  },
  {
    label: "application termination does not initiate hidden config or provider writes",
    text: files.app + "\n" + files.store,
    passed: !/applicationShouldTerminate|configureAutosaveFlusher|flushPendingAutosaves/.test(files.app),
  },
  {
    label: "confirmed config apply uses the serialized confirmed mutation lane without autosave",
    text: files.store + "\n" + files.confirmedMutationLane,
    passed: /private let confirmedMutationLane = ConfirmedMutationLane\(\)/.test(files.store)
      && /func applyClaudeSettingsSave\([\s\S]*?confirmedMutationLane\.perform/.test(files.store)
      && /final class ConfirmedMutationLane[\s\S]*?func perform<Result>/.test(files.confirmedMutationLane)
      && !/submit(?:Provider|Config)Autosave/.test(files.store)
      && /deinit[\s\S]*?lane\.shutdown\(\)/.test(files.store),
  },
  {
    label: "formal summaries and global search expose stable full-access controls",
    text: [
      files.sidebar,
      files.batchSkillOperation,
      files.taskCockpit,
      files.skillManager,
      files.detailPrimitives,
      files.content,
    ].join("\n"),
    passed: [
      "batch-toggle-items.show-all",
      "task-cockpit-candidates.show-all",
      "task-cockpit-context.show-all",
      "markdown-table.show-all",
      "global-search.skills.view-all",
      "global-search.sessions.view-all",
      "global-search.config-history.view-all",
    ].every((identifier) => [
      files.sidebar,
      files.batchSkillOperation,
      files.taskCockpit,
      files.skillManager,
      files.detailPrimitives,
      files.content,
    ].some((source) => source.includes(identifier)))
      && /private struct TaskCockpitTechnicalDiagnosticsView:[\s\S]*?TaskCockpitCandidateList\([\s\S]*?routeCandidates[\s\S]*?TaskCockpitCandidateList\([\s\S]*?agentCandidates[\s\S]*?TaskCockpitCandidateList\([\s\S]*?skillCandidates[\s\S]*?TaskCockpitContextList\([\s\S]*?gapRows[\s\S]*?TaskCockpitContextList\([\s\S]*?blockerRows[\s\S]*?TaskCockpitEvidenceList\([\s\S]*?evidenceReferences[\s\S]*?TaskCockpitSafetyList\(/.test(files.taskCockpit)
      && /private struct SkillManagerSelectableRow:[\s\S]*?\.accessibilityLabel\(title\)[\s\S]*?\.accessibilityValue/.test(files.skillManager),
  },
  {
    label: "main shell uses NavigationSplitView",
    text: files.content,
    pattern: /NavigationSplitView(?:\([\s\S]*?\))?\s*{/,
  },
  {
    label: "list pages use a unified window toolbar with global search and sidebar-local selectors",
    text: files.content + "\n" + files.advancedWorkspace + "\n" + files.uiOptimization,
    passed: /static let unifiedToolbar = UnifiedToolbarPresentation\(\)[\s\S]*?static let listPage = ListPagePresentation\(\)[\s\S]*?static let sidebarShell = SidebarShellPresentation\(\)/.test(files.uiOptimization)
      && /struct UnifiedToolbarPresentation:[\s\S]*?spansEntireWindow = true[\s\S]*?searchPlacement = UnifiedToolbarSearchPlacement\.globalTrailing[\s\S]*?collapsesAtScrollEdge = true[\s\S]*?settingsActionUsesSystemSettingsLink = true/.test(files.uiOptimization)
      && /struct ListPagePresentation:[\s\S]*?filterStyle = ListPageFilterStyle\.capsule[\s\S]*?searchScope = ListPageSearchScope\.localList[\s\S]*?rowStyle = ListPageRowStyle\.whiteCard[\s\S]*?minimumCardRowHeight = 58[\s\S]*?cardRowSpacing = 8/.test(files.uiOptimization)
      && /ZStack\(alignment:\s*\.topTrailing\)[\s\S]*?if shouldShowGlobalSearchResultsOverlay[\s\S]*?globalSearchResultsOverlay[\s\S]*?pinnedWindowChromeControls/.test(files.content)
      && /private var appShell:\s*some View\s*\{[\s\S]*?navigationShell[\s\S]*?\n\s*\}/.test(files.content)
      && /private var pinnedWindowChromeControls:\s*some View\s*\{[\s\S]*?WindowChromeTitlebarAccessory\s*\{[\s\S]*?WindowChromeToolbarControls\([\s\S]*?text:\s*\$globalSearchText,[\s\S]*?isSearchFocused:\s*\$isGlobalSearchFocused,[\s\S]*?showsSearchResults:\s*\$showsGlobalSearchResults,[\s\S]*?onSubmit:\s*selectFirstGlobalSearchResult[\s\S]*?\.frame\(width:\s*0,\s*height:\s*0\)[\s\S]*?\.allowsHitTesting\(false\)[\s\S]*?\.accessibilityHidden\(true\)[\s\S]*?\.zIndex\(10\)/.test(files.content)
      && /@State private var isGlobalSearchFocused = false[\s\S]*?@State private var showsGlobalSearchResults = false/.test(files.content)
      && /private var globalSearchResultsOverlay:[\s\S]*?GlobalSearchResultsOverlay\([\s\S]*?query:\s*trimmedGlobalSearchText,[\s\S]*?results:\s*globalSearchResults,[\s\S]*?kindCounts:\s*store\.appSearchResult\.kindCounts[\s\S]*?onViewAll:\s*showAllGlobalSearchResults[\s\S]*?selectGlobalSearchResult\(result\)[\s\S]*?WindowChromeToolbarMetrics\.searchResultsTrailingPadding/.test(files.content)
      && /SecondarySidebarView\(columnVisibility:\s*columnVisibility\)/.test(files.advancedWorkspace)
      && !/WindowChromeAgentControl/.test(files.content)
      && !/WindowChromeProjectControl/.test(files.content)
      && /@State private var columnVisibility:\s*NavigationSplitViewVisibility = \.all[\s\S]*?NavigationSplitView\(columnVisibility:\s*\$columnVisibility\)/.test(files.content)
      && !/WindowChromeTitlebarInstaller|WindowChromeChildWindow|WindowChromeTitlebarLayout/.test(files.content)
      && !/secondarySidebarHeaderWidth/.test(files.content)
      && !/ToolbarItem\(placement:\s*\.primaryAction\)\s*\{\s*WindowChromeToolbarControls/.test(files.content)
      && !/ToolbarItem\(placement:\s*\.navigation\)\s*\{\s*WindowChromeToolbarControls/.test(files.content)
      && !/private struct WindowChromeTopBarBackdrop/.test(files.content)
      && /private struct WindowChromeTitlebarAccessory<Content:\s*View>:\s*NSViewRepresentable[\s\S]*?NSTitlebarAccessoryViewController\(\)[\s\S]*?accessory\.layoutAttribute = \.right[\s\S]*?window\.addTitlebarAccessoryViewController\(accessory\)[\s\S]*?removeTitlebarAccessoryViewController\(at:\s*index\)/.test(files.content)
      && /private final class FirstMouseTitlebarAccessoryContainer:\s*NSView[\s\S]*?intrinsicContentSize[\s\S]*?acceptsFirstMouse/.test(files.content)
      && !/WindowChromeTopGlass|windowChromeTopGlass|PassthroughWindowChromeHostingView|topGlassHeight/.test(files.content)
      && /private enum WindowChromeToolbarMetrics[\s\S]*?controlHeight:\s*CGFloat = 32[\s\S]*?agentWidth:\s*CGFloat = 146[\s\S]*?projectWidth:\s*CGFloat = 210[\s\S]*?titlebarTrailingPadding:\s*CGFloat = 28[\s\S]*?searchWidth = CGFloat\(UIOptimizationPresentation\.unifiedToolbar\.idealGlobalSearchWidth\)[\s\S]*?searchResultsWidth:\s*CGFloat = 460[\s\S]*?searchResultsMinHeight:\s*CGFloat = 180[\s\S]*?static var trailingWidth:[\s\S]*?static var totalWidth:[\s\S]*?static var accessoryWidth:[\s\S]*?static var searchResultsTrailingPadding:/.test(files.content)
      && /private struct WindowChromeToolbarControls:\s*View[\s\S]*?HStack\(spacing:\s*8\)\s*\{\s*TitlebarProjectPickerControl\(isCompact:\s*false\)[\s\S]*?\.frame\(width:\s*projectWidth,\s*height:\s*controlHeight,\s*alignment:\s*\.leading\)\s*TitlebarAgentSelectorControl\(\)[\s\S]*?\.frame\(width:\s*agentWidth,\s*height:\s*controlHeight,\s*alignment:\s*\.leading\)\s*WindowChromeTrailingControls\([\s\S]*?text:\s*\$text[\s\S]*?private var controlHeight:\s*CGFloat \{ WindowChromeToolbarMetrics\.controlHeight \}[\s\S]*?private var agentWidth:\s*CGFloat \{ WindowChromeToolbarMetrics\.agentWidth \}[\s\S]*?private var projectWidth:\s*CGFloat \{ WindowChromeToolbarMetrics\.projectWidth \}/.test(files.content)
      && !extractStructBody(files.content, "WindowChromeToolbarControls").includes("Divider()")
      && !/private struct WindowChromeToolbarControls:[\s\S]*?columnVisibility|isPrimarySidebarCollapsed/.test(files.content)
      && !/GlassEffectContainer|glassEffect\(/.test(files.content)
      && !/\.toolbar\s*\{[\s\S]*?ToolbarItem\(placement:\s*\.navigation\)[\s\S]*?TitlebarAgentSelectorControl\(\)/.test(files.content)
      && /private struct TitlebarAgentSelectorControl:\s*View[\s\S]*?isPopoverPresented\.toggle\(\)[\s\S]*?TitlebarAgentSelectorLabel\([\s\S]*?\.popover\(isPresented:\s*\$isPopoverPresented[\s\S]*?ForEach\(SkillAgentFilter\.managementCases\)[\s\S]*?store\.agentFilter = filter/.test(files.content)
      && /private struct TitlebarProjectPickerControl:\s*View[\s\S]*?Button\s*\{[\s\S]*?isPopoverPresented\.toggle\(\)[\s\S]*?\.popover\(isPresented:\s*\$isPopoverPresented[\s\S]*?Button\s*\{[\s\S]*?chooseProject\(\)[\s\S]*?await store\.previewClearRecentProjects\(\)[\s\S]*?ForEach\(store\.recentProjectContexts\)[\s\S]*?selectProject\([\s\S]*?recentProjectPath\(context\)[\s\S]*?await store\.removeRecentProject\([\s\S]*?revealActiveProject\(\)[\s\S]*?await store\.previewClearProject\(\)[\s\S]*?private func selectProject\([\s\S]*?store\.requestProjectSelection\(/.test(files.content)
      && /struct SecondarySidebarView:[\s\S]*?let columnVisibility:\s*NavigationSplitViewVisibility[\s\S]*?List\(selection:\s*\$store\.selectedSidebarSelection\)[\s\S]*?\.padding\(\.top,\s*50\)[\s\S]*?\.ignoresSafeArea\(\.container,\s*edges:\s*\.top\)[\s\S]*?GeometryReader \{ proxy in[\s\S]*?SecondarySidebarHeaderWidthPreferenceKey\.self[\s\S]*?\.allowsHitTesting\(false\)[\s\S]*?\.navigationTitle\(UIStrings\.appWindowTitle\)/.test(files.sidebar)
      && !/\.overlay\(alignment:\s*\.topLeading\)[\s\S]*?SecondarySidebarHeaderChrome/.test(files.sidebar)
      && !/ToolbarItemGroup\(placement:\s*\.automatic\)[\s\S]*?Global/.test(files.content)
      && /private struct WindowChromeTrailingControls:[\s\S]*?private let searchWidth = WindowChromeToolbarMetrics\.searchWidth[\s\S]*?private var controls:[\s\S]*?HStack\(alignment:\s*\.center,\s*spacing:\s*6\)[\s\S]*?GlobalWindowSearchControl\([\s\S]*?WindowChromeSettingsControl\(\)[\s\S]*?\.frame\(height:\s*32,\s*alignment:\s*\.center\)/.test(files.content)
      && !extractStructBody(files.content, "WindowChromeTrailingControls").includes("WindowChromeHelpButton()")
      && /private struct GlobalWindowSearchControl:[\s\S]*?@Binding var isSearchFocused:[\s\S]*?@Binding var showsResults:[\s\S]*?WindowChromeSearchTextField\([\s\S]*?placeholder:\s*UIStrings\.text\("toolbar\.globalSearch"[\s\S]*?\) \{ focused in[\s\S]*?showsResults = !trimmedText\.isEmpty[\s\S]*?onChange\(of:\s*text\)[\s\S]*?showsResults = !trimmedText\.isEmpty[\s\S]*?Image\(systemName:\s*"magnifyingglass"\)[\s\S]*?\.windowChromeGlassCapsule\(\)/.test(files.content)
      && !/private struct GlobalWindowSearchControl:[\s\S]*?\.popover\(isPresented:\s*resultsPopoverBinding/.test(files.content)
      && /private struct WindowChromeSearchTextField:\s*NSViewRepresentable[\s\S]*?FirstMouseNSTextField[\s\S]*?isBordered = false[\s\S]*?drawsBackground = false[\s\S]*?focusRingType = \.none[\s\S]*?controlTextDidChange[\s\S]*?control\([\s\S]*?insertNewline/.test(files.content)
      && /private final class FirstMouseNSTextField:\s*NSTextField[\s\S]*?acceptsFirstMouse/.test(files.content)
      && /private struct GlobalSearchResultsOverlay:[\s\S]*?let results:\s*\[AppSearchItem\][\s\S]*?let kindCounts:\s*\[AppSearchKindCount\][\s\S]*?let isLoading:\s*Bool[\s\S]*?ForEach\(AppSearchItemKind\.allCases[\s\S]*?let kindResults = results\.filter \{ \$0\.kind == kind \}[\s\S]*?Text\("\\\(kind\.title\) \\\(count\(for:\s*kind\)\)"\)[\s\S]*?Button\(viewAllTitle\(for:\s*kind\)\)[\s\S]*?ForEach\(kindResults\)[\s\S]*?WindowChromeToolbarMetrics\.searchResultsMinHeight[\s\S]*?WindowChromeToolbarMetrics\.searchResultsWidth[\s\S]*?\.fill\(\.regularMaterial\)/.test(files.content)
      && !/NativeToolbarSearchField/.test(files.content)
      && !/NSSearchField/.test(files.content)
      && !/ToolbarAvatarView/.test(files.content)
      && !/AppBrandToolbarItem/.test(files.content)
      && !/GlobalToolbarSearchField/.test(files.content)
      && !/toolbar\.new/.test(files.content)
      && !/isSkillManagerSheetPresented/.test(files.content)
      && /private struct WindowChromeHelpButton:[\s\S]*?isShowingHelp\.toggle\(\)[\s\S]*?questionmark\.circle[\s\S]*?frame\(width:\s*30,\s*height:\s*30\)[\s\S]*?\.windowChromeGlassCircle\(\)[\s\S]*?\.popover\(isPresented:\s*\$isShowingHelp[\s\S]*?help\.summary[\s\S]*?help\.privacy[\s\S]*?help\.documentation/.test(files.content)
      && !/NSApp\.orderFrontStandardAboutPanel\(nil\)/.test(files.content)
      && /private struct WindowChromeSettingsControl:[\s\S]*?if #available\(macOS 14\.0,\s*\*\)[\s\S]*?SettingsLink[\s\S]*?settingsLabel[\s\S]*?Button\(action:\s*openSettingsFallback\)/.test(files.content)
      && /private struct WindowChromeSettingsControl:[\s\S]*?\.windowChromeGlassCircle\(\)[\s\S]*?gearshape[\s\S]*?frame\(width:\s*30,\s*height:\s*30\)[\s\S]*?openSettingsFallback\(\)[\s\S]*?showPreferencesWindow/.test(files.content)
      && /private extension View[\s\S]*?func windowChromeGlassCapsule\(\)[\s\S]*?Color\(nsColor:\s*\.controlBackgroundColor\)\.opacity\(0\.72\)[\s\S]*?func windowChromeGlassCircle\(\)[\s\S]*?Color\(nsColor:\s*\.controlBackgroundColor\)\.opacity\(0\.72\)/.test(files.content)
      && /struct SecondarySidebarHeaderWidthPreferenceKey:\s*PreferenceKey[\s\S]*?struct SecondarySidebarHeaderChrome:\s*View[\s\S]*?let availableWidth:\s*CGFloat[\s\S]*?let agentLeading = agentLeadingInset\(for:\s*availableWidth\)[\s\S]*?let projectLeading = projectLeadingInset\(for:\s*availableWidth,\s*agentLeading:\s*agentLeading\)[\s\S]*?let agentFrame = CGRect\([\s\S]*?let projectFrame = CGRect\([\s\S]*?ZStack\(alignment:\s*\.topLeading\)[\s\S]*?SecondarySidebarAgentHeaderControl\(\)[\s\S]*?\.offset\(x:\s*agentLeading,[\s\S]*?SecondarySidebarProjectHeaderControl\(isCompact:\s*isPrimarySidebarCollapsed\)[\s\S]*?\.offset\(x:\s*projectLeading,[\s\S]*?\.contentShape\([\s\S]*?SecondarySidebarHeaderHitShape\([\s\S]*?agentFrame:\s*agentFrame,[\s\S]*?projectFrame:\s*projectFrame[\s\S]*?private func agentLeadingInset\(for availableWidth:[\s\S]*?private func projectLeadingInset\(for availableWidth:[\s\S]*?private struct SecondarySidebarHeaderHitShape:\s*Shape[\s\S]*?path\.addRoundedRect\(in:\s*agentFrame[\s\S]*?path\.addRoundedRect\(in:\s*projectFrame/.test(files.sidebar)
      && /struct SecondarySidebarAgentHeaderControl:\s*View[\s\S]*?SecondarySidebarAgentSelectorMenu\(\)[\s\S]*?frame\(minWidth:\s*126,\s*idealWidth:\s*148,\s*maxWidth:\s*158/.test(files.sidebar)
      && /struct SecondarySidebarProjectHeaderControl:\s*View[\s\S]*?let isCompact:\s*Bool[\s\S]*?SecondarySidebarProjectPickerMenu\(isCompact:\s*isCompact\)[\s\S]*?minWidth:\s*isCompact \? 36 : 42[\s\S]*?idealWidth:\s*isCompact \? 36 : 140[\s\S]*?maxWidth:\s*isCompact \? 36 : 152/.test(files.sidebar)
      && !/SecondarySidebarProjectPickerMenu\(isCompact:\s*true\)[\s\S]*?frame\(maxWidth:\s*\.infinity,\s*alignment:\s*\.trailing\)/.test(files.sidebar)
      && /private struct SecondarySidebarAgentSelectorMenu:[\s\S]*?Menu\s*\{[\s\S]*?ForEach\(SkillAgentFilter\.managementCases\)[\s\S]*?store\.agentFilter = filter[\s\S]*?SecondarySidebarAgentSelectorLabel\([\s\S]*?shortTitle\(for:\s*store\.agentFilter\)[\s\S]*?\.accessibilityValue\(store\.agentFilter\.title\)/.test(files.sidebar)
      && /private struct SecondarySidebarAgentSelectorLabel:[\s\S]*?AgentIconBadge\(filter:\s*filter,\s*size:\s*24\)[\s\S]*?Image\(systemName:\s*"chevron\.up\.chevron\.down"\)[\s\S]*?\.frame\(minWidth:\s*126[\s\S]*?\.secondarySidebarHeaderControlCapsule\(\)/.test(files.sidebar)
      && /private struct SecondarySidebarProjectPickerMenu:[\s\S]*?Menu\s*\{[\s\S]*?Label\(UIStrings\.chooseProject,\s*systemImage:\s*"folder\.badge\.plus"\)[\s\S]*?Section\(UIStrings\.recentProjects\)[\s\S]*?await store\.setProject\([\s\S]*?recentProjectTitle\(context\)[\s\S]*?await store\.removeRecentProject\([\s\S]*?await store\.previewClearRecentProjects\(\)[\s\S]*?Label\(UIStrings\.revealInFinder,[\s\S]*?arrow\.up\.forward\.app[\s\S]*?Label\(UIStrings\.clearProject,[\s\S]*?xmark\.circle[\s\S]*?SecondarySidebarProjectPickerLabel\([\s\S]*?title:\s*projectTitle[\s\S]*?return UIStrings\.toolbarNoProjectSelected[\s\S]*?private var projectHelp:[\s\S]*?DisplayText\.privacyPath\(rootPath,\s*privacyModeEnabled:\s*true\)/.test(files.sidebar)
      && /private struct SecondarySidebarProjectPickerMenu:[\s\S]*?let isCompact:\s*Bool[\s\S]*?SecondarySidebarProjectPickerLabel\([\s\S]*?isCompact:\s*isCompact/.test(files.sidebar)
      && /private struct SecondarySidebarProjectPickerLabel:[\s\S]*?let isCompact:\s*Bool[\s\S]*?if isCompact[\s\S]*?collapsedLabel[\s\S]*?ViewThatFits\(in:\s*\.horizontal\)[\s\S]*?expandedLabel[\s\S]*?collapsedLabel[\s\S]*?\.secondarySidebarHeaderControlCapsule\(\)[\s\S]*?\.secondarySidebarHeaderControlCircle\(\)/.test(files.sidebar)
      && !/ToolbarContextSummary/.test(files.content)
      && /private var globalSearchResults:\s*\[AppSearchItem\][\s\S]*?store\.appSearchResult\.query == trimmedGlobalSearchText[\s\S]*?return store\.appSearchResult\.items/.test(files.content)
      && /\.onChange\(of:\s*trimmedGlobalSearchText\)[\s\S]*?store\.updateAppSearch\(query:\s*query\)/.test(files.content)
      && /func updateAppSearch\(query:[\s\S]*?AppSearchIndex\([\s\S]*?sessionSummaries:\s*summaries[\s\S]*?\.search\(query:\s*query,\s*limitPerKind:\s*Self\.globalSearchLimitPerKind\)[\s\S]*?func selectAppSearchItem\(_ item:\s*AppSearchItem\) async[\s\S]*?case \.skill:[\s\S]*?setSidebarSelection\(\.skill\(skill\.id\)\)[\s\S]*?case \.session:[\s\S]*?localSessionPreviewResult = localSessionPreviewResult\.ensuringSession\(session\)[\s\S]*?selectLocalSession\(session,\s*origin:\s*\.navigation\)[\s\S]*?case \.configHistory:[\s\S]*?ensureConfigSnapshot\(snapshot\)[\s\S]*?selectConfigSnapshot\(snapshot\)/.test(files.store)
      && !/private func performAppSearch\(query:[\s\S]*?service\.searchApp\(/.test(files.store)
      && /private func selectGlobalSearchResult\(_ result:\s*AppSearchItem\)[\s\S]*?await store\.selectAppSearchItem\(result\)[\s\S]*?globalSearchText = ""/.test(files.content)
      && !/private func applyGlobalSearch/.test(files.content),
  },
  {
    label: "secondary sidebar header preserves liquid glass behind a toolchain guard",
    text: files.sidebar,
    passed: /func secondarySidebarHeaderControlCapsule\(\)[\s\S]*?#if compiler\(>=6\.2\)[\s\S]*?glassEffect\(\.regular\.interactive\(\), in:\s*Capsule\(\)\)[\s\S]*?#else[\s\S]*?secondarySidebarHeaderControlFallback\(shape:\s*Capsule\(\)\)[\s\S]*?#endif/.test(files.sidebar)
      && /func secondarySidebarHeaderControlCircle\(\)[\s\S]*?#if compiler\(>=6\.2\)[\s\S]*?glassEffect\(\.regular\.interactive\(\), in:\s*Circle\(\)\)[\s\S]*?#else[\s\S]*?secondarySidebarHeaderControlFallback\(shape:\s*Circle\(\)\)[\s\S]*?#endif/.test(files.sidebar)
      && /func secondarySidebarHeaderControlFallback<S:\s*Shape>\(shape:\s*S\)[\s\S]*?background\(Color\.agentCopilotPanelBackground,\s*in:\s*shape\)[\s\S]*?shape[\s\S]*?\.stroke\(Color\.secondary\.opacity\(0\.12\), lineWidth:\s*1\)/.test(files.sidebar),
  },
  {
    label: "startup prewarm shows only loading progress before revealing the app shell",
    text: files.content + "\n" + files.store + "\n" + files.localizable,
    passed: /ZStack\(alignment:\s*\.topTrailing\)\s*{[\s\S]*?appShell[\s\S]*?\.opacity\(store\.startupLoadingState == nil \? 1 : 0\)[\s\S]*?\.allowsHitTesting\(store\.startupLoadingState == nil\)[\s\S]*?if let state = store\.startupLoadingState[\s\S]*?AppStartupLoadingView\(state:\s*state\)[\s\S]*?pinnedWindowChromeControls/.test(files.content)
      && /\.task\s*{[\s\S]*?await store\.loadAppStartupDataIfNeeded\(\)[\s\S]*?}/.test(files.content)
      && /private struct AppStartupLoadingView:[\s\S]*?Text\(state\.message\)[\s\S]*?ProgressView\(value:\s*state\.progress\)[\s\S]*?\.background\(Color\.agentCopilotWindowBackground\)/.test(files.content)
      && !/if store\.status == nil && store\.skills\.isEmpty[\s\S]*?await store\.reload\(\)/.test(files.content)
      && /struct AppStartupLoadingState:[\s\S]*?let message: String[\s\S]*?let progress: Double/.test(files.storePresentationModels)
      && /@Published private\(set\) var startupLoadingState:[\s\S]*?UIStrings\.startupPreparingLoading/.test(files.store)
      && /@Published private\(set\) var hasCompletedStartupLoad = false/.test(files.store)
      && /func loadAppStartupDataIfNeeded\(\) async[\s\S]*?try await refreshCollections\(includeSupplementalData:\s*false,\s*includeAIProviderStatus:\s*false\)[\s\S]*?scheduleStartupSupplementalLoads\(/.test(files.store)
      && !/func loadAppStartupDataIfNeeded\(\) async[\s\S]*?catalog\.getSkill|func loadAppStartupDataIfNeeded\(\) async[\s\S]*?loadSelectedDetail/.test(files.store)
      && /private func scheduleStartupSupplementalLoads\([\s\S]*?loadLocalSessions:\s*true[\s\S]*?loadAgentConfigDocuments:\s*true[\s\S]*?forceProviderObservability:\s*false/.test(files.store)
      && /private func schedulePostRefreshSupplementalLoads\([\s\S]*?await self\.loadAIProviderStatusIfNeeded\(\)[\s\S]*?await self\.refreshSelectedAgentLocalSessionsIfNeeded\(\)[\s\S]*?await self\.loadCurrentAgentConfigDocumentsIfNeeded\(agent:\s*requestedAgentFilter\.rawValue\)[\s\S]*?await self\.loadLLMPromptRuns\(\)[\s\S]*?await self\.loadProviderObservabilityDuringRefresh\(force:\s*forceProviderObservability\)/.test(files.store)
      && /"startup\.catalog" = "Loading catalog data\.\.\."/.test(files.localizable),
  },
  {
    label: "primary and secondary sidebar columns have bounded native widths",
    text: files.content + "\n" + files.advancedWorkspace + "\n" + files.uiOptimization,
    passed: /struct SidebarShellPresentation:[\s\S]*?let width = 260/.test(files.uiOptimization)
      && /minimumSecondaryColumnWidth = 360[\s\S]*?idealSecondaryColumnWidth = 400[\s\S]*?maximumSecondaryColumnWidth = 520/.test(files.uiOptimization)
      && /SidebarView\(\)[\s\S]*?UIOptimizationPresentation\.sidebarShell\.width[\s\S]*?UIOptimizationPresentation\.sidebarShell\.width[\s\S]*?UIOptimizationPresentation\.sidebarShell\.width/.test(files.content)
      && /SecondarySidebarView\(columnVisibility:\s*columnVisibility\)[\s\S]*?UIOptimizationPresentation\.skillList\.minimumSecondaryColumnWidth[\s\S]*?UIOptimizationPresentation\.skillList\.idealSecondaryColumnWidth[\s\S]*?UIOptimizationPresentation\.skillList\.maximumSecondaryColumnWidth/.test(files.advancedWorkspace),
  },
  {
    label: "session data is prewarmed at startup without route-triggered root scans",
    text: files.content + "\n" + files.storeSurface,
    passed: /private func scheduleStartupSupplementalLoads\([\s\S]*?loadLocalSessions:\s*true/.test(files.storeSurface)
      && /func refreshSelectedAgentLocalSessionsIfNeeded\(\)\s*async[\s\S]*?refreshLocalSessionSnapshot\(reason:\s*\.sourceChanged\)/.test(files.storeSurface)
      && !/\.task\(id:\s*store\.selectedAgentLocalSessionRefreshKey\)/.test(files.content),
  },
  {
    label: "primary sidebar exposes exactly the three product workspaces",
    text: files.sidebar + "\n" + files.content + "\n" + files.app,
    passed: /List\(selection:\s*routeSelection\)[\s\S]*?Section\(UIStrings\.text\("sidebar\.primaryNavigation"/.test(files.sidebar)
      && !/ProjectContextControls\(\)/.test(files.sidebar)
      && /PrimarySidebarRow\([\s\S]*?\.tag\(AppRoute\.overview\)[\s\S]*?PrimarySidebarRow\([\s\S]*?\.tag\(AppRoute\.skills\)[\s\S]*?PrimarySidebarRow\([\s\S]*?\.tag\(AppRoute\.sessions\)/.test(files.sidebar)
      && /private var routeSelection:\s*Binding<AppRoute\?>[\s\S]*?store\.selectAppRoute\(route\)/.test(files.sidebar)
      && !extractStructBody(files.sidebar, "SidebarView").includes(".tag(AppRoute.advanced)")
      && !extractStructBody(files.sidebar, "SidebarView").includes("SidebarFooterToolRow")
      && !extractStructBody(files.sidebar, "SidebarView").includes("TaskPreflightPreviewSheet")
      && !extractStructBody(files.sidebar, "SidebarView").includes("SkillPackageManagerSheet")
      && /case \.overview:[\s\S]*?ProjectOverviewView\([\s\S]*?case \.skills:[\s\S]*?SkillsWorkspaceView\(\)[\s\S]*?case \.sessions:[\s\S]*?SessionsWorkspaceView\(\)[\s\S]*?case \.advanced:[\s\S]*?AdvancedWorkspaceView\(columnVisibility:\s*columnVisibility\)/.test(files.content)
      && /struct AdvancedWorkspaceView:[\s\S]*?HSplitView[\s\S]*?SecondarySidebarView\(columnVisibility:\s*columnVisibility\)[\s\S]*?AdvancedConfigurationDetailView\(\)/.test(files.advancedWorkspace)
      && /CommandMenu\(UIStrings\.text\("menu\.navigate",\s*"Navigate"\)\)[\s\S]*?selectAppRoute\(\.overview\)[\s\S]*?selectAppRoute\(\.skills\)[\s\S]*?selectAppRoute\(\.sessions\)/.test(files.app),
  },
  {
    label: "Skills workspace is aggregate-first, explicit-selection, and capability-gated",
    text: [
      files.content,
      files.skillsWorkspace,
      files.skillsWorkspaceList,
      files.skillAggregateDetail,
      files.skillsWorkspaceModel,
      files.skillManagerEntryContext,
      files.skillWorkspaceStore,
      files.skillManagerModel,
    ].join("\n"),
    passed: /case \.skills:[\s\S]*?SkillsWorkspaceView\(\)/.test(files.content)
      && /struct SkillsWorkspaceView:[\s\S]*?HSplitView[\s\S]*?SkillsWorkspaceListView\([\s\S]*?workspace:\s*store\.skillWorkspaceStore[\s\S]*?SkillAggregateDetailView\(/.test(files.skillsWorkspace)
      && /availableConfigActions:\s*availableConfigActions\(for:\s*aggregate\)[\s\S]*?onConfigAction:[\s\S]*?openConfigFlow\(action,\s*aggregate:\s*aggregate\)/.test(files.skillsWorkspace)
      && /private func availableConfigActions\([\s\S]*?toggleDisabledReason\(for:\s*\$0\) == nil[\s\S]*?actions\.insert\(\.enable\)[\s\S]*?actions\.insert\(\.disable\)/.test(files.skillsWorkspace)
      && /private func openConfigFlow\([\s\S]*?prepareSkillTogglePreview\([\s\S]*?instanceIDs:\s*aggregate\.instanceIDs[\s\S]*?isConfigOperationPresented = true/.test(files.skillsWorkspace)
      && /private struct SkillPackageManagerSheet:[\s\S]*?WorkflowSheetShell\([\s\S]*?SkillManagerPanel\([\s\S]*?entryContext:\s*entryContext/.test(files.skillsWorkspace)
      && /SkillManagerPackageTarget\([\s\S]*?\.uniqueBestMatch\(in:\s*cachedInventoryItems\)/.test(files.skillsWorkspace)
      && /SkillManagerInventoryActionPolicy\.availableActions\(for:\s*item\)/.test(files.skillsWorkspace)
      && /onContextualIntelligence:[\s\S]*?SkillContextualIntelligenceSheet/.test(files.skillsWorkspace)
      && /ContextualIntelligenceView\([\s\S]*?providerGateMessage:\s*providerGateMessage[\s\S]*?previewSkillReview\([\s\S]*?sendSkillReview\(/.test(files.skillsWorkspace)
      && /static let orderedViews:[\s\S]*?\.needsAttention,[\s\S]*?\.project,[\s\S]*?\.global,[\s\S]*?\.all/.test(files.skillsWorkspaceModel)
      && /List\(selection:\s*selectionBinding\)[\s\S]*?ForEach\(presentation\.rows\)/.test(files.skillsWorkspaceList)
      && /case \.answer:[\s\S]*?answerLayer[\s\S]*?case \.evidence:[\s\S]*?evidenceLayer[\s\S]*?case \.advanced:[\s\S]*?advancedLayer/.test(files.skillAggregateDetail)
      && /private func normalizeSelection[\s\S]*?selectedAggregateID = nil/.test(files.skillWorkspaceStore)
      && /static func add\([\s\S]*?static func packageDetail\([\s\S]*?static func update\([\s\S]*?static func remove\(/.test(files.skillManagerEntryContext),
  },
  {
    label: "retired duplicate product surfaces and state are absent",
    passed: presentRetiredPresentationPaths.length === 0
      && !/TaskPreflightPreviewSheet|TaskCockpitHistoryRecord|taskCockpitHistory|selectedTaskCockpitHistoryID/.test(files.taskCockpit + "\n" + files.taskCockpitModel + "\n" + files.storeSurface)
      && !/selectedDetailSection|selectedSkillDetail|selectedSkillEvents|loadSelectedDetail|loadMoreSkillEvents/.test(files.storeSurface + "\n" + files.content)
      && !/SidebarFooterToolRow|SidebarFooterToolButton/.test(files.sidebar)
      && !/struct SkillPackageManagerSheet/.test(files.sidebar)
      && !/"taskCockpit\.history\.|"menu\.showTaskCockpit"|"sidebar\.preflight|"skillManager\.sidebar/.test(files.localizable + "\n" + files.localizableZh),
  },
  {
    label: "session detail is Summary-first, fixed-snapshot paged, and copy-only",
    text: files.sessionWorkspaceDetail + "\n" + files.sessionWorkspaceDetailModel,
    passed: /enum SessionWorkspaceDetailLayer:[\s\S]*?case summary[\s\S]*?case timeline[\s\S]*?case evidence/.test(files.sessionWorkspaceDetailModel)
      && /case \.summary:[\s\S]*?summaryLayer\(presentation\)[\s\S]*?case \.timeline:[\s\S]*?timelineLayer\(presentation\)[\s\S]*?case \.evidence:[\s\S]*?evidenceLayer\(presentation\)/.test(files.sessionWorkspaceDetail)
      && /presentation\.timelineItems[\s\S]*?ListCompletenessFooter\([\s\S]*?state:\s*messageCompleteness[\s\S]*?onLoadMore:\s*onLoadMore[\s\S]*?onLoadAll:\s*onLoadAll[\s\S]*?onCancel:\s*onCancel/.test(files.sessionWorkspaceDetail)
      && /case \.notPreviewed:[\s\S]*?Button\(action:\s*onPreviewResume\)/.test(files.sessionWorkspaceDetail)
      && /case \.supported\(let command\):[\s\S]*?onCopyResumeCommand\(command\)/.test(files.sessionWorkspaceDetail)
      && /never launches a terminal, runs this command, or translates the session to another agent/.test(files.sessionWorkspaceDetail)
      && !/NSWorkspace|Process\(|Terminal|osascript/.test(files.sessionWorkspaceDetail),
  },
  {
    label: "Sessions workspace list is project-first, cache-filtered, paged, and explicit-selection",
    text: [
      files.sessionsWorkspaceList,
      files.sessionsWorkspaceModel,
      files.sessionWorkspaceStore,
      files.localSessionCache,
    ].join("\n"),
    passed: /struct SessionWorkspaceListPresentation:[\s\S]*?static let orderedAgents:[\s\S]*?\.claudeCode,[\s\S]*?\.codex,[\s\S]*?\.opencode,[\s\S]*?\.pi,[\s\S]*?\.hermes,[\s\S]*?\.openclaw/.test(files.sessionsWorkspaceModel)
      && /struct SessionWorkspaceCriteria:[\s\S]*?scope:\s*LocalSessionScopeFilter = \.project/.test(files.sessionWorkspaceStore)
      && /struct SessionsWorkspaceListView:[\s\S]*?List\(selection:\s*selectionBinding\)[\s\S]*?ForEach\(presentation\.projectGroups\)[\s\S]*?Section\(group\.title\)[\s\S]*?ForEach\(group\.rows\)/.test(files.sessionsWorkspaceList)
      && /let onSetCriteria:\s*\(SessionWorkspaceCriteria\) -> Void[\s\S]*?let onSelectSession:\s*\(String\?\) -> Void[\s\S]*?let onLoadNext:\s*\(\) async -> Void[\s\S]*?let onRefresh:\s*\(\) async -> Void/.test(files.sessionsWorkspaceList)
      && /ListCompletenessFooter\([\s\S]*?state:\s*presentation\.completeness[\s\S]*?onLoadMore:[\s\S]*?onLoadAll:[\s\S]*?accessibilityIdentifierPrefix:\s*"sessions-workspace"/.test(files.sessionsWorkspaceList)
      && /private var selectionBinding:\s*Binding<String\?>[\s\S]*?get:\s*\{\s*workspace\.selectedSessionID\s*\}[\s\S]*?set:\s*onSelectSession/.test(files.sessionsWorkspaceList)
      && !/onAppear[\s\S]*?onSelectSession/.test(files.sessionsWorkspaceList)
      && /enum SessionWorkspaceProjectGroupKind:[\s\S]*?case selectedProject[\s\S]*?case otherProject[\s\S]*?case unmatched/.test(files.sessionsWorkspaceModel)
      && /struct SessionWorkspaceRowPresentation:[\s\S]*?let title:[\s\S]*?let agentLabel:[\s\S]*?let projectLabel:[\s\S]*?let timeLabel:[\s\S]*?let intentExcerpt:/.test(files.sessionsWorkspaceModel)
      && !/redactedPath|sourceKind|contentHash/.test(files.sessionsWorkspaceList),
  },
  {
    label: "advanced secondary sidebar exposes configuration only",
    text: files.sidebar + "\n" + files.advancedWorkspace,
    passed: /struct SecondarySidebarView:[\s\S]*?List\(selection:\s*\$store\.selectedSidebarSelection\)[\s\S]*?ConfigSidebarPanel\(\)/.test(files.sidebar)
      && /struct AdvancedWorkspaceView:[\s\S]*?SecondarySidebarView\(columnVisibility:\s*columnVisibility\)[\s\S]*?AdvancedConfigurationDetailView\(\)[\s\S]*?store\.openAdvancedConfiguration\(\)/.test(files.advancedWorkspace)
      && !/SessionSidebarPanel|SkillSidebarPanel|AgentProfileSidebarRow|SidebarSelection\.agentWorkspace|switch store\.sidebarContentMode/.test(files.sidebar),
  },
  {
    label: "config sidebar exposes scope filtering, clean operation support, disabled skills, and selectable config history",
    text: files.sidebar + "\n" + files.agentConfigWorkspace,
    passed: /var visibleConfigDocuments:[\s\S]*?currentAgentConfigDocuments[\s\S]*?document\.agent == agentFilter\.rawValue[\s\S]*?configScopeFilter\.includes\(document\)[\s\S]*?configDocumentMatchesSidebarQuery\(document\)[\s\S]*?lhs\.scope\.lowercased\(\)\.contains\("project"\)[\s\S]*?localizedStandardCompare/.test(files.storeSurface)
      && /private struct ConfigSidebarPanel:[\s\S]*?private var selectedConfigDocuments:[\s\S]*?store\.visibleConfigDocuments[\s\S]*?Section\s*{[\s\S]*?configToolbar[\s\S]*?Section\(UIStrings\.currentConfigFile\)[\s\S]*?ForEach\(selectedConfigDocuments,\s*id:\s*\\\.target\)[\s\S]*?ConfigCurrentDocumentSidebarRow\([\s\S]*?document:\s*document[\s\S]*?isSelected:\s*store\.selectedSidebarSelection == \.configDocument\(document\.target\)[\s\S]*?store\.selectConfigDocument\(document\)[\s\S]*?Supported operations[\s\S]*?ConfigOperationRow\(title:\s*UIStrings\.scan[\s\S]*?ConfigOperationRow\(title:\s*UIStrings\.writableConfig[\s\S]*?UIStrings\.agentConfigSkillEnablement[\s\S]*?ConfigDisabledSkillSummaryRow\(skills:\s*disabledSkills\)[\s\S]*?ForEach\(selectedSnapshots\)[\s\S]*?ConfigSnapshotSidebarRow\([\s\S]*?store\.selectedSidebarSelection == \.configSnapshot\(snapshot\.id\)[\s\S]*?store\.selectConfigSnapshot\(snapshot\)/.test(files.sidebar)
      && /private var configToolbar:[\s\S]*?let layout = UIOptimizationPresentation\.skillList[\s\S]*?VStack\(alignment:\s*\.leading,\s*spacing:\s*8\)[\s\S]*?HStack\(alignment:\s*\.center,\s*spacing:\s*CGFloat\(layout\.filterControlSpacing\)\)[\s\S]*?configScopePicker[\s\S]*?configRefreshButton\([\s\S]*?width:\s*CGFloat\(layout\.sortDirectionButtonWidth\),[\s\S]*?height:\s*CGFloat\(layout\.filterControlHeight\)[\s\S]*?configSearchField/.test(files.sidebar)
      && !/private var configToolbar:[\s\S]*?ViewThatFits\(in:\s*\.horizontal\)[\s\S]*?private var configScopePicker/.test(files.sidebar)
      && !/private var configToolbar:[\s\S]*?Spacer\(minLength:[\s\S]*?private var configScopePicker/.test(files.sidebar)
      && /private var configScopePicker:[\s\S]*?SkillFilterMenuPicker\([\s\S]*?title:\s*UIStrings\.scope[\s\S]*?selection:\s*\$store\.configScopeFilter[\s\S]*?options:\s*AgentConfigScopeFilter\.allCases[\s\S]*?expands:\s*false/.test(files.sidebar)
      && /private var configSearchField:[\s\S]*?SidebarSearchField\([\s\S]*?sidebar\.config\.search[\s\S]*?\$store\.configSidebarSearchText/.test(files.sidebar)
      && /private func configRefreshButton\(width:\s*CGFloat,\s*height:\s*CGFloat\)[\s\S]*?await store\.refreshSelectedAgentConfigData\(\)[\s\S]*?Image\(systemName:\s*"arrow\.clockwise"\)[\s\S]*?\.frame\(width:\s*width,\s*height:\s*height\)[\s\S]*?\.buttonStyle\(\.plain\)/.test(files.sidebar)
      && /@Published var configSidebarSearchText/.test(files.store)
      && /private var disabledSkills:[\s\S]*?AgentConfigDisplay\.disabledSkills\(for:\s*store\.agentFilter,\s*store:\s*store\)/.test(files.sidebar)
      && /private struct ConfigDisabledSkillSummaryRow:[\s\S]*?UIStrings\.agentConfigDisabledSkillsCount\(skills\.count\)[\s\S]*?UIStrings\.agentConfigDisabledSkillsEmpty/.test(files.sidebar)
      && /private struct ConfigCurrentDocumentSidebarRow:[\s\S]*?let isSelected:\s*Bool[\s\S]*?DisplayText\.scope\(document\.scope\)[\s\S]*?AgentConfigDisplay\.pathSummary\(document\.target\)[\s\S]*?document\.exists \? UIStrings\.existingFile : UIStrings\.willCreateFile[\s\S]*?optimizedSidebarSelection\(isSelected:\s*isSelected\)/.test(files.sidebar)
      && /private struct ConfigSnapshotSidebarRow:[\s\S]*?item\.timeText[\s\S]*?item\.scopeText[\s\S]*?item\.capturedText[\s\S]*?item\.targetSummary/.test(files.sidebar)
      && /\.task\(id:\s*store\.selectedAgentConfigRefreshKey\)[\s\S]*?await store\.loadSelectedAgentConfigDataIfNeeded\(\)/.test(files.sidebar)
      && /func loadSelectedAgentConfigDataIfNeeded\(\) async[\s\S]*?loadAgentConfigSnapshotsIfNeeded[\s\S]*?loadCurrentAgentConfigDocumentsIfNeeded/.test(files.store)
      && /func loadCurrentAgentConfigDocumentsIfNeeded\(agent requestedAgent:[\s\S]*?force:\s*false/.test(files.store)
      && !/\.onChange\(of:\s*store\.configScopeFilter\)[\s\S]*?loadAgentConfigSnapshots|\.onChange\(of:\s*store\.configScopeFilter\)[\s\S]*?loadCurrentAgentConfigDocuments/.test(files.sidebar)
      && /case configDocument\(String\)[\s\S]*?case \.configOverview,\s*\.configDocument,\s*\.configSnapshot/.test(files.sidebarSelection)
      && /var selectedConfigDocument:[\s\S]*?case let \.configDocument\(target\)[\s\S]*?currentAgentConfigDocuments\.first[\s\S]*?func selectConfigDocument\(_ document:[\s\S]*?guard selectedSidebarSelection != \.configDocument\(document\.target\)[\s\S]*?selectedSidebarSelection = \.configDocument\(document\.target\)/.test(files.storeProjectionHelpers + "\n" + files.store)
      && /AgentConfigOverviewDetailPanel\(selectedDocument:\s*store\.selectedConfigDocument\)[\s\S]*?let selectedDocument:[\s\S]*?if let selectedDocument[\s\S]*?currentAgentConfigSection\(documents:\s*\[selectedDocument\]\)/.test(files.agentConfigWorkspace)
      && !/AgentConfigCapabilityCard|AgentConfigDisabledSkillsPanel/.test(files.agentConfigWorkspace)
      && !/Text\(capability\?\.status/.test(files.sidebar + "\n" + files.agentConfigWorkspace),
  },
  {
    label: "current config detail uses the unified single-card editor layout",
    text: files.agentConfigWorkspace + "\n" + files.uiOptimization,
    passed: /static let configEditor = ConfigEditorPresentation\(\)/.test(files.uiOptimization)
      && /struct ConfigEditorPresentation:[\s\S]*?usesSingleCodeCard = true[\s\S]*?showsLineNumbers = true[\s\S]*?usesCompactToolbarActions = true[\s\S]*?primarySaveButtonVisible = true[\s\S]*?autosaveEnabled = false/.test(files.uiOptimization)
      && /private struct ConfigCodeCard<[\s\S]*?PrivacyPathText\(path:\s*path[\s\S]*?toolbar\(\)[\s\S]*?content\(\)[\s\S]*?\.nativePanelSurface\(\)/.test(files.agentConfigWorkspace)
      && /private struct ConfigCodeToolbar:[\s\S]*?UIStrings\.reload[\s\S]*?UIStrings\.formatJSON[\s\S]*?isSensitiveVisible \? "eye\.slash" : "eye"[\s\S]*?onReveal/.test(files.agentConfigWorkspace)
      && /private struct AgentCurrentConfigDocumentsSection:[\s\S]*?ConfigCodeCard\([\s\S]*?title:\s*UIStrings\.currentConfigFile[\s\S]*?path:\s*primaryDocument\?\.target[\s\S]*?statusText:\s*primaryDocument\?\.exists == true \? UIStrings\.existingFile : UIStrings\.willCreateFile[\s\S]*?ConfigCodeToolbar\([\s\S]*?onReload:\s*reload[\s\S]*?JSONSyntaxHighlightedText\(content:\s*displayedContent\)/.test(files.agentConfigWorkspace)
      && /Button\(UIStrings\.text\("settings\.agentConfig\.save",\s*"Save"\)\)[\s\S]*?previewConfigSave\(\)/.test(files.agentConfigWorkspace)
      && !/Read-only agent config previews intentionally keep the save slot disabled/.test(files.agentConfigWorkspace)
      && !/AgentCurrentConfigDocumentPane/.test(files.agentConfigWorkspace)
      && !/UIStrings\.agentConfigReadOnlyPreview/.test(files.agentConfigWorkspace)
      && !/UIStrings\.agentConfigReadOnlyBoundary/.test(files.agentConfigWorkspace),
  },
  {
    label: "config raw editing is confirmation-gated and read-only previews use syntax highlighting",
    text: files.agentConfigWorkspace + "\n" + files.store + "\n" + files.uiStrings + "\n" + files.localizable + "\n" + files.localizableZh,
    passed: /@State private var isConfirmingConfigEdit = false/.test(files.agentConfigWorkspace)
      && !/@State private var configAutosaveTask: Task<Void,\s*Never>\?/.test(files.agentConfigWorkspace)
      && /private func toggleSensitiveEditing\(\)[\s\S]*?if revealsSensitiveConfig \{[\s\S]*?revealsSensitiveConfig = false[\s\S]*?\} else \{[\s\S]*?isConfirmingConfigEdit = true[\s\S]*?\}/.test(files.agentConfigWorkspace)
      && /\.confirmationDialog\(\s*UIStrings\.agentConfigEditConfirmationTitle,[\s\S]*?isPresented:\s*\$isConfirmingConfigEdit[\s\S]*?Button\(UIStrings\.agentConfigShowSensitive,\s*role:\s*\.destructive\)[\s\S]*?revealsSensitiveConfig = true[\s\S]*?Text\(UIStrings\.agentConfigEditConfirmationMessage\)/.test(files.agentConfigWorkspace)
      && /if revealsSensitiveConfig \{[\s\S]*?JSONLineNumberedEditor\(text:\s*displayedDraft\)[\s\S]*?\} else \{[\s\S]*?JSONSyntaxHighlightedText\(content:\s*displayedDraft\.wrappedValue\)/.test(files.agentConfigWorkspace)
      && /private func handleConfigDraftChange\(\)[\s\S]*?invalidateConfigPreview\(\)[\s\S]*?store\.clearSettingsFeedback\(\)/.test(files.agentConfigWorkspace)
      && /private func invalidateConfigPreview\(\)[\s\S]*?configPreviewTask\?\.cancel\(\)[\s\S]*?configConfirmationToApply = nil[\s\S]*?store\.invalidateConfigSavePreview\(\)/.test(files.agentConfigWorkspace)
      && !extractFunctionBody(files.agentConfigWorkspace, "handleConfigDraftChange").includes("Task.sleep")
      && !extractFunctionBody(files.agentConfigWorkspace, "handleConfigDraftChange").includes("saveClaudeSettings")
      && !extractFunctionBody(files.agentConfigWorkspace, "handleConfigDraftChange").includes("previewClaudeSettingsSave")
      && /@Published private\(set\) var configMutationState:\s*ConfigMutationState = \.idle/.test(files.store)
      && /func previewClaudeSettingsSave\(content:[\s\S]*?service\.previewClaudeSettingsSave/.test(files.store)
      && /func applyClaudeSettingsSave\([\s\S]*?service\.saveClaudeSettings/.test(files.store)
      && /confirmationDialog\([\s\S]*?"settings\.agentConfig\.confirmSave"[\s\S]*?applyClaudeSettingsSave/.test(files.agentConfigWorkspace)
      && /private struct JSONSyntaxHighlightedText:[\s\S]*?ForEach\(Array\(Self\.lines\(in:\s*content\)\.enumerated\(\)\)[\s\S]*?Text\(Self\.highlighted[\s\S]*?NSRegularExpression[\s\S]*?AttributedString/.test(files.agentConfigWorkspace)
      && /private struct JSONLineNumberedEditor:[\s\S]*?ConfigLineNumberColumn\(lineCount:\s*lineCount\)[\s\S]*?TextEditor\(text:\s*\$text\)/.test(files.agentConfigWorkspace)
      && /static var agentConfigEditConfirmationTitle/.test(files.uiStrings)
      && /static var agentConfigEditConfirmationMessage/.test(files.uiStrings)
      && /static var formatJSON/.test(files.uiStrings)
      && /"settings\.agentConfig\.editConfirmation\.title"/.test(files.localizable)
      && /"settings\.agentConfig\.editConfirmation\.message"/.test(files.localizableZh)
      && /"settings\.agentConfig\.confirmSave"/.test(files.localizable)
      && /"settings\.agentConfig\.awaitingConfirmation"/.test(files.localizableZh)
      && /"action\.formatJSON"/.test(files.localizableZh),
  },
  {
    label: "config draft stays local until preview and explicit confirmation",
    text: files.agentConfigWorkspace + "\n" + files.store,
    passed: /@State private var draft = ""/.test(files.agentConfigWorkspace)
      && /private func hydrateConfigDraftFromStore\([\s\S]*?invalidateConfigPreview\(\)[\s\S]*?let incoming = store\.claudeSettings\?\.content \?\? ""[\s\S]*?draft = incoming/.test(files.agentConfigWorkspace)
      && /\.onChange\(of:\s*store\.claudeSettings\)[\s\S]*?reconcileConfigDraftFromStore\(revealsSensitive:\s*revealsSensitiveConfig\)/.test(files.agentConfigWorkspace)
      && /\.task\(id:\s*store\.selectedAgentConfigRefreshKey\)[\s\S]*?hydrateConfigDraftFromStore\(\)/.test(files.agentConfigWorkspace)
      && /private func previewConfigSave\(\)[\s\S]*?let candidate = draft[\s\S]*?previewClaudeSettingsSave\(content:\s*candidate\)[\s\S]*?draft == candidate[\s\S]*?confirmation\?\.content == candidate/.test(files.agentConfigWorkspace)
      && /private func resetDraftFromStore\([\s\S]*?hydrateConfigDraftFromStore[\s\S]*?clearSettingsFeedback/.test(files.agentConfigWorkspace)
      && !/submitConfigAutosave|configAutosaveCoordinator|configAutosaveDraft/.test(files.agentConfigWorkspace + "\n" + files.store),
  },
  {
    label: "rollback preview presentation rejects stale view tasks across replacement selection and disappearance",
    text: files.agentConfigWorkspace,
    passed: /@State private var previewPresentation = RollbackPreviewPresentationState<SnapshotRollbackPreviewRecord>\(\)/.test(files.agentConfigWorkspace)
      && /@State private var previewLoadTask: Task<Void,\s*Never>\?/.test(files.agentConfigWorkspace)
      && /private func invalidatePreviewLoad\(selectedSnapshotID:\s*String\?\)[\s\S]*?previewLoadTask\?\.cancel\(\)[\s\S]*?previewPresentation\.invalidate\(selectedSnapshotID:\s*selectedSnapshotID\)[\s\S]*?store\.clearRollbackConfirmation\(\)/.test(files.agentConfigWorkspace)
      && /\.onChange\(of:\s*snapshot\.id\)[\s\S]*?invalidatePreviewLoad\(selectedSnapshotID:\s*snapshot\.id\)/.test(files.agentConfigWorkspace)
      && /\.onDisappear[\s\S]*?invalidatePreviewLoad\(selectedSnapshotID:\s*nil\)/.test(files.agentConfigWorkspace)
      && /private func loadPreview\(\)[\s\S]*?previewLoadTask\?\.cancel\(\)[\s\S]*?let request = previewPresentation\.begin\(snapshotID:\s*snapshot\.id\)[\s\S]*?previewLoadTask = Task \{ @MainActor in[\s\S]*?store\.previewRollback\(snapshotID:\s*request\.snapshotID\)[\s\S]*?previewPresentation\.publish\(preview:\s*loadedPreview,\s*for:\s*request\)[\s\S]*?previewPresentation\.publish\(errorMessage:\s*error\.localizedDescription,\s*for:\s*request\)/.test(files.agentConfigWorkspace)
      && !/@State private var preview:\s*SnapshotRollbackPreviewRecord\?/.test(files.agentConfigWorkspace)
      && !/@State private var previewError:\s*String\?/.test(files.agentConfigWorkspace),
  },
  {
    label: "config sensitive toggle keeps Hide enabled while Reveal remains binding and busy gated",
    text: files.agentConfigWorkspace,
    passed: /private var sensitiveTogglePolicy:\s*AgentConfigSensitiveTogglePolicy[\s\S]*?AgentConfigSensitiveTogglePolicy\([\s\S]*?isSensitiveVisible:\s*revealsSensitiveConfig,[\s\S]*?hasLoadedDocument:\s*store\.claudeSettings != nil,[\s\S]*?hasWritableBinding:\s*hasWritableConfigBinding,[\s\S]*?isLoading:\s*store\.isLoadingSettings,[\s\S]*?isSaving:\s*store\.isSavingSettings/.test(files.agentConfigWorkspace)
      && /isRevealDisabled:\s*sensitiveTogglePolicy\.isDisabled/.test(files.agentConfigWorkspace)
      && !/isRevealDisabled:\s*store\.isLoadingSettings[\s\S]*?store\.claudeSettings != nil && !hasWritableConfigBinding/.test(files.agentConfigWorkspace),
  },
  {
    label: "high-priority accessibility and localized summary fixes are present",
    text: files.detailPrimitives + "\n" + files.agentConfigWorkspace + "\n" + files.skillManager + "\n" + files.sidebar + "\n" + files.content + "\n" + files.sessionWorkspaceDetail + "\n" + files.uiStrings,
    passed: /struct SummaryChip:[\s\S]*?\.accessibilityElement\(children:\s*\.combine\)[\s\S]*?\.accessibilityLabel\(title\)[\s\S]*?\.accessibilityValue\(value\)/.test(files.detailPrimitives)
      && /private struct AgentConfigAgentIcon:[\s\S]*?\.accessibilityLabel\(filter\.title\)/.test(files.agentConfigWorkspace)
      && /private struct SkillManagerSelectableRow:[\s\S]*?\.accessibilityLabel\(title\)[\s\S]*?\.accessibilityValue/.test(files.skillManager)
      && /private struct SecondarySidebarProjectPickerMenu:[\s\S]*?\.accessibilityLabel\(UIStrings\.text\("project\.chooseMenu"/.test(files.sidebar),
  },
  {
    label: "UIStrings falls back through native localization before defaults",
    text: files.uiStrings,
    pattern: /static func text\(_ key:\s*String,\s*_ defaultValue:\s*String\) -> String[\s\S]*?if let value = localizedStrings\(\)\[key\][\s\S]*?Bundle\.main\.localizedString\(forKey:\s*key,\s*value:\s*nil,\s*table:\s*nil\)[\s\S]*?nativeValue != key[\s\S]*?return defaultValue/,
  },
  {
    label: "primary sidebar rows stay lightweight and metric-free",
    text: files.sidebar,
    passed: /private struct PrimarySidebarRow:[\s\S]*?Image\(systemName:\s*systemImage\)[\s\S]*?Text\(title\)[\s\S]*?if let subtitle[\s\S]*?Text\(subtitle\)/.test(files.sidebar)
      && !/SidebarNavigationMetricPill|sessionCardMetrics|skillCardMetrics|configCardMetrics/.test(
        extractStructBody(files.sidebar, "SidebarView"),
      ),
  },
  {
    label: "LLM Markdown output normalizes collapsed provider markdown tables",
    text: files.markdownRender,
    pattern: /normalizeMarkdownBlocks\(in text:[\s\S]*?normalizeInlineMarkdownBreaks\(in: line\)[\s\S]*?normalizeInlineTableRows\(in text:[\s\S]*?\| \|[\s\S]*?isStandaloneTableLine/,
  },
  {
    label: "LLM Markdown output unwraps whole-response markdown fences",
    text: files.markdownRender,
    pattern: /static func renderableText\(from text: String\)[\s\S]*?hasPrefix\("```"\)[\s\S]*?\["markdown", "md", "gfm"\]\.contains\(language\)[\s\S]*?return body/,
  },
  {
    label: "LLM prompt instructions require the evidence envelope and forbid tables and fences",
    text: `${files.serviceLLM}\n${files.serviceRust}`,
    passed: /llm_output_language_instruction\(params\.app_language\.as_deref\(\)\)/.test(files.serviceLLM)
      && /Required output: return only the exact JSON response envelope[\s\S]*?without Markdown fences or extra text/.test(files.serviceLLM)
      && /Do not use Markdown tables[\s\S]*?Do not wrap the answer in fenced code blocks/.test(files.serviceRust),
  },
  {
    label: "task cockpit panel lives in a dedicated module file",
    text: files.taskCockpit,
    pattern: /struct TaskCockpitPanel:[\s\S]*?TaskCockpitResultView[\s\S]*?TaskCockpitSafetyList/,
  },
  {
    label: "task cockpit keeps progressive staged feedback inside technical diagnostics",
    text: files.taskCockpit,
    pattern: /struct TaskCockpitStageProgressView:[\s\S]*?TaskCockpitProgressSnapshot\([\s\S]*?ForEach\(snapshot\.stageRows\)[\s\S]*?TaskCockpitStageTile\(row:[\s\S]*?accessibilityIdentifier\(AppAccessibilityID\.taskCockpitStageProgress\)[\s\S]*?struct TaskCockpitTechnicalDiagnosticsView:[\s\S]*?TaskCockpitStageProgressView\(/,
  },
  {
    label: "task cockpit task input uses an AX-settable multiline TextField",
    text: files.taskInput,
    pattern: /struct TaskInputTextEditor:[\s\S]*?TextField\(placeholder,\s*text:\s*\$text,\s*axis:\s*\.vertical\)[\s\S]*?\.lineLimit\(3\.\.\.5\)[\s\S]*?\.frame\([\s\S]*?minHeight:\s*Self\.minHeight[\s\S]*?maxHeight:\s*Self\.maxHeight[\s\S]*?\.accessibilityIdentifier\(AppAccessibilityID\.taskCockpitInput\)/,
  },
  {
    label: "task cockpit input model preserves raw text and trims only for submit state",
    text: files.taskInputModel,
    pattern: /struct TaskInputModel:[\s\S]*?let rawText:[\s\S]*?rawText\.trimmingCharacters\(in:\s*\.whitespacesAndNewlines\)[\s\S]*?var canSubmit:[\s\S]*?!trimmedText\.isEmpty/,
  },
  {
    label: "task cockpit build button remains explicit and input-gated",
    text: files.taskCockpit,
    pattern: /Button\s*{[\s\S]*?onBuild\(\)[\s\S]*?\.disabled\(isPreviewingPrompt \|\| isBuilding \|\| !inputModel\.canSubmit \|\| selectedAgentIDs\.isEmpty \|\| providerGateMessage != nil\)/,
  },
  {
    label: "detail presentation primitives live in a dedicated module file",
    text: files.detailPrimitives,
    pattern: /struct SafetyPill:[\s\S]*?struct SummaryChip:[\s\S]*?struct RoutingInlineList:[\s\S]*?struct MetadataRow:/,
  },
  {
    label: "dense disclosure list caps visible rows and reveals overflow",
    text: files.detailPrimitives,
    pattern: /struct DenseDisclosureList<Item,\s*RowContent:\s*View>:[\s\S]*?visibleLimit:\s*Int = 6[\s\S]*?ForEach\(Array\(items\[0\.\.<visibleEnd\]\.enumerated\(\)\),\s*id:\s*\\\.offset\)[\s\S]*?DisclosureGroup\(isExpanded:\s*\$isExpanded\)[\s\S]*?items\.dropFirst\(visibleLimit\)[\s\S]*?private var visibleEnd:\s*Int[\s\S]*?min\(visibleLimit,\s*items\.count\)/,
  },
  {
    label: "dense inline evidence lists are counted, collapsible, and screenshot-safe",
    text: files.detailPrimitives,
    pattern: /struct RoutingInlineList:[\s\S]*?DenseCountBadge\(count:\s*values\.count\)[\s\S]*?DenseDisclosureList\(values,\s*visibleLimit:\s*3,\s*spacing:\s*3\)[\s\S]*?PrivacyEvidenceLabel\(value:\s*value,\s*systemImage:\s*systemImage,\s*font:\s*\.caption,\s*lineLimit:\s*2\)/,
  },
  {
    label: "task cockpit evidence list is compact, expandable, and screenshot-safe",
    text: files.taskCockpit,
    pattern: /struct TaskCockpitEvidenceList:[\s\S]*?ExpandableSummaryList\([\s\S]*?evidence,[\s\S]*?visibleLimit:\s*6,[\s\S]*?task-cockpit-evidence\.show-all[\s\S]*?PrivacyEvidenceText\(value:\s*source,\s*font:\s*\.caption2,\s*lineLimit:\s*1\)[\s\S]*?PrivacyEvidenceText\(value:\s*item\.detail,\s*font:\s*\.caption,\s*lineLimit:\s*nil\)/,
  },
  {
    label: "window titlebar accessory owns agent and project selection",
    passed: /List\(selection:\s*routeSelection\)[\s\S]*?Section\(UIStrings\.text\("sidebar\.primaryNavigation"/.test(files.sidebar)
      && !/ProjectContextControls\(\)/.test(files.sidebar)
      && !/AgentWorkspaceHeader\(\)/.test(files.sidebar)
      && !/private struct AgentWorkspaceHeader/.test(files.sidebar)
      && !/private struct AgentSelectorMenu/.test(files.sidebar)
      && !/WindowChromeAgentControl/.test(files.content)
      && !/WindowChromeProjectControl/.test(files.content)
      && !/WindowChromeTitlebarInstaller|WindowChromeChildWindow|WindowChromeTitlebarLayout/.test(files.content)
      && /SecondarySidebarView\(columnVisibility:\s*columnVisibility\)/.test(files.advancedWorkspace)
      && !/secondarySidebarHeaderWidth/.test(files.content)
      && /ZStack\(alignment:\s*\.topTrailing\)[\s\S]*?globalSearchResultsOverlay[\s\S]*?pinnedWindowChromeControls/.test(files.content)
      && /private var pinnedWindowChromeControls:\s*some View\s*\{[\s\S]*?WindowChromeTitlebarAccessory\s*\{[\s\S]*?WindowChromeToolbarControls\([\s\S]*?text:\s*\$globalSearchText,[\s\S]*?isSearchFocused:\s*\$isGlobalSearchFocused,[\s\S]*?showsSearchResults:\s*\$showsGlobalSearchResults,[\s\S]*?onSubmit:\s*selectFirstGlobalSearchResult[\s\S]*?\.frame\(width:\s*0,\s*height:\s*0\)[\s\S]*?\.zIndex\(10\)/.test(files.content)
      && !/ToolbarItem\(placement:\s*\.primaryAction\)\s*\{\s*WindowChromeToolbarControls/.test(files.content)
      && !/ToolbarItem\(placement:\s*\.navigation\)\s*\{\s*WindowChromeToolbarControls/.test(files.content)
      && !/private struct WindowChromeTopBarBackdrop/.test(files.content)
      && /private struct WindowChromeTitlebarAccessory<Content:\s*View>:\s*NSViewRepresentable[\s\S]*?accessory\.layoutAttribute = \.right[\s\S]*?FirstMouseTitlebarAccessoryContainer/.test(files.content)
      && !/WindowChromeTopGlass|windowChromeTopGlass|PassthroughWindowChromeHostingView|topGlassHeight/.test(files.content)
      && /private struct WindowChromeToolbarControls:\s*View[\s\S]*?HStack\(spacing:\s*8\)\s*\{\s*TitlebarProjectPickerControl\(isCompact:\s*false\)[\s\S]*?\.frame\(width:\s*projectWidth,\s*height:\s*controlHeight,\s*alignment:\s*\.leading\)\s*TitlebarAgentSelectorControl\(\)[\s\S]*?\.frame\(width:\s*agentWidth,\s*height:\s*controlHeight,\s*alignment:\s*\.leading\)\s*WindowChromeTrailingControls\([\s\S]*?text:\s*\$text/.test(files.content)
      && !extractStructBody(files.content, "WindowChromeToolbarControls").includes("Divider()")
      && !/\.toolbar\s*\{[\s\S]*?ToolbarItem\(placement:\s*\.navigation\)[\s\S]*?TitlebarAgentSelectorControl\(\)/.test(files.content)
      && /struct SecondarySidebarView:[\s\S]*?let columnVisibility:\s*NavigationSplitViewVisibility[\s\S]*?List\(selection:\s*\$store\.selectedSidebarSelection\)[\s\S]*?\.padding\(\.top,\s*50\)[\s\S]*?GeometryReader \{ proxy in[\s\S]*?SecondarySidebarHeaderWidthPreferenceKey\.self[\s\S]*?\.allowsHitTesting\(false\)/.test(files.sidebar)
      && !/\.overlay\(alignment:\s*\.topLeading\)[\s\S]*?SecondarySidebarHeaderChrome/.test(files.sidebar)
      && /private struct TitlebarAgentSelectorControl:\s*View[\s\S]*?isPopoverPresented\.toggle\(\)[\s\S]*?TitlebarAgentSelectorLabel\([\s\S]*?\.popover\(isPresented:\s*\$isPopoverPresented[\s\S]*?ForEach\(SkillAgentFilter\.managementCases\)[\s\S]*?store\.agentFilter = filter[\s\S]*?\.accessibilityValue\(store\.agentFilter\.title\)/.test(files.content)
      && /private struct TitlebarAgentIconBadge:[\s\S]*?var size:\s*CGFloat = 28[\s\S]*?frame\(width:\s*imageSize,\s*height:\s*imageSize\)[\s\S]*?frame\(width:\s*size,\s*height:\s*size\)/.test(files.content)
      && /private struct TitlebarProjectPickerControl:\s*View[\s\S]*?Button\s*\{[\s\S]*?isPopoverPresented\.toggle\(\)[\s\S]*?\.popover\(isPresented:\s*\$isPopoverPresented[\s\S]*?ForEach\(store\.recentProjectContexts\)[\s\S]*?selectProject\([\s\S]*?NSOpenPanel\(\)[\s\S]*?store\.requestProjectSelection\([\s\S]*?NSWorkspace\.shared\.activateFileViewerSelecting/.test(files.content)
      && !/return 224/.test(files.content)
      && !/SecondarySidebarProjectPickerMenu\(isCompact:\s*true\)[\s\S]*?frame\(maxWidth:\s*\.infinity,\s*alignment:\s*\.trailing\)/.test(files.sidebar)
      && !/ProjectContextToolbarControl/.test(files.sidebar)
      && !/store\.selectedSidebarSelection\s*=\s*\.agentWorkspace/.test(files.sidebar)
      && !/\.tag\(SidebarSelection\.agentWorkspace\)/.test(files.sidebar),
  },
  {
    label: "secondary sidebar project menu owns merged project selection and actions",
    text: files.sidebar,
    pattern: /private struct SecondarySidebarProjectPickerMenu:[\s\S]*?Menu\s*\{[\s\S]*?Label\(UIStrings\.chooseProject,\s*systemImage:\s*"folder\.badge\.plus"\)[\s\S]*?Section\(UIStrings\.recentProjects\)[\s\S]*?await store\.setProject\([\s\S]*?await store\.removeRecentProject\([\s\S]*?await store\.previewClearRecentProjects\(\)[\s\S]*?Label\(UIStrings\.revealInFinder,[\s\S]*?arrow\.up\.forward\.app[\s\S]*?Label\(UIStrings\.clearProject,[\s\S]*?xmark\.circle[\s\S]*?SecondarySidebarProjectPickerLabel\([\s\S]*?\.menuStyle\(\.button\)[\s\S]*?\.buttonStyle\(\.plain\)[\s\S]*?private struct SecondarySidebarProjectPickerLabel:[\s\S]*?ViewThatFits\(in:\s*\.horizontal\)/,
  },
  {
    label: "titlebar project popover keeps compact clear action in the recent header",
    text: files.content,
    passed: (() => {
      const body = extractStructBody(files.content, "TitlebarProjectPickerControl")
      const recentTitle = body.indexOf("Text(UIStrings.recentProjects)")
      const compactClear = body.indexOf("Text(UIStrings.clearRecentProjectsCompact)")
      const recentRows = body.indexOf("ForEach(store.recentProjectContexts)")
      return /HStack\(spacing:\s*8\)[\s\S]*?Text\(UIStrings\.recentProjects\)[\s\S]*?Spacer\(minLength:\s*8\)[\s\S]*?Button\(role:\s*\.destructive\)/.test(body)
        && recentTitle >= 0
        && compactClear > recentTitle
        && recentRows > compactClear
        && body.includes("Task { await store.previewClearRecentProjects() }")
        && !body.includes('Label(UIStrings.clearRecentProjects, systemImage: "trash.slash")')
    })(),
  },
  {
    label: "snapshot preview sheet has bounded width",
    text: files.configSnapshotPreview,
    pattern: /\.frame\(width:\s*980,\s*height:\s*680\)/,
  },
  {
    label: "snapshot preview panes are scrollable for long content",
    text: files.configSnapshotPreview,
    pattern: /ScrollView\(\[\.vertical,\s*\.horizontal\]\)/,
  },
  {
    label: "settings window has stable minimum dimensions",
    text: files.settings,
    passed: /UIOptimizationPresentation\.settings\.minimumWidth/.test(files.settings)
      && /UIOptimizationPresentation\.settings\.idealWidth/.test(files.settings)
      && /UIOptimizationPresentation\.settings\.minimumHeight/.test(files.settings)
      && /UIOptimizationPresentation\.settings\.idealHeight/.test(files.settings)
      && /UIOptimizationPresentation\.settings\.sidebarWidth/.test(files.settings)
      && /minimumWidth = 760[\s\S]*?idealWidth = 860[\s\S]*?minimumHeight = 620[\s\S]*?idealHeight = 680/.test(files.uiOptimization),
  },
  {
    label: "settings window uses sidebar navigation and close-only window controls",
    text: files.settings + "\n" + files.uiOptimization + "\n" + files.localizable + "\n" + files.localizableZh,
    passed: /enum SettingsTab:[\s\S]*?CaseIterable[\s\S]*?case appearance[\s\S]*?case provider[\s\S]*?case providerObservability[\s\S]*?case advanced/.test(files.settings)
      && /HStack\(spacing:\s*0\)[\s\S]*?settingsSidebar[\s\S]*?Divider\(\)[\s\S]*?selectedSettingsPane/.test(files.settings)
      && /private var settingsSidebar:[\s\S]*?ForEach\(SettingsTab\.allCases\)[\s\S]*?SettingsSidebarItem/.test(files.settings)
      && /private var selectedSettingsPane:[\s\S]*?switch selectedSettingsTab[\s\S]*?case \.providerObservability:[\s\S]*?ProviderObservabilitySettingsPanel\(\)[\s\S]*?case \.advanced:[\s\S]*?advancedSection/.test(files.settings)
      && /@AppStorage\(SettingsNavigation\.selectionStorageKey\)[\s\S]*?selectedSettingsTab:\s*SettingsTab/.test(files.settings)
      && /SettingsNavigation\.providerObservabilityRequested[\s\S]*?selectedSettingsTab = \.providerObservability/.test(files.settings)
      && /private struct SettingsWindowConfigurator:[\s\S]*?window\.title = UIStrings\.settingsWindowTitle[\s\S]*?window\.styleMask\.remove\(\.miniaturizable\)[\s\S]*?standardWindowButton\(\.miniaturizeButton\)\?\.isHidden = true[\s\S]*?standardWindowButton\(\.zoomButton\)\?\.isHidden = true/.test(files.settings)
      && /navigationStyle = SettingsNavigationStyle\.sidebar[\s\S]*?usesDedicatedSettingsScene = true[\s\S]*?windowControlPolicy = SettingsWindowControlPolicy\.closeOnly[\s\S]*?primarySaveButtonsVisible = false[\s\S]*?sidebarWidth = 190/.test(files.uiOptimization)
      && /"settings\.window\.title"/.test(files.localizable)
      && /"settings\.nav\.appearance\.subtitle"/.test(files.localizable)
      && /"settings\.nav\.provider\.subtitle"/.test(files.localizableZh)
      && !/TabView\(selection:\s*\$selectedSettingsTab\)/.test(files.settings),
  },
  {
    label: "settings pages use unified headers and compact sections",
    text: files.settings,
    passed: /private struct SettingsPageHeader/.test(files.settings)
      && /private struct SettingsSectionCard/.test(files.settings)
      && /SettingsPageHeader\([\s\S]*?title:\s*UIStrings\.appearanceSettings/.test(files.settings)
      && /SettingsSectionCard\([\s\S]*?title:\s*UIStrings\.themeSettings/.test(files.settings)
      && /SettingsPageHeader\([\s\S]*?title:\s*UIStrings\.aiProviderSettings/.test(files.settings)
      && /SettingsSectionCard\(title:\s*UIStrings\.text\("settings\.aiProvider\.connection"/.test(files.settings)
      && /SettingsSectionCard\(title:\s*UIStrings\.text\("settings\.aiProvider\.limits"/.test(files.settings)
      && /SettingsSectionCard\(title:\s*UIStrings\.text\("settings\.aiProvider\.credentialSafety"/.test(files.settings)
      && /SettingsPageHeader\([\s\S]*?title:\s*UIStrings\.text\("settings\.advanced",\s*"Advanced"\)/.test(files.settings)
      && /DetailMetricGrid\(maxColumns:\s*3/.test(files.settings)
      && /Picker\(UIStrings\.themeSelection,[\s\S]*?ForEach\(AppTheme\.allCases\)[\s\S]*?\.pickerStyle\(\.segmented\)[\s\S]*?\.labelsHidden\(\)/.test(files.settings)
      && /Picker\(UIStrings\.languageSelection,[\s\S]*?\.pickerStyle\(\.segmented\)[\s\S]*?\.labelsHidden\(\)/.test(files.settings)
      && /Picker\(UIStrings\.llmProvider,[\s\S]*?\.pickerStyle\(\.segmented\)[\s\S]*?\.labelsHidden\(\)/.test(files.settings),
  },
  {
    label: "advanced mechanisms are discoverable without becoming primary navigation",
    text: [
      files.content,
      files.advancedWorkspace,
      files.settings,
      files.settingsNavigation,
      files.store,
      files.sidebar,
      files.privacyPath,
      files.agentConfigWorkspace,
    ].join("\n"),
    passed: !extractStructBody(files.sidebar, "SidebarView").includes(".tag(AppRoute.advanced)")
      && /case \.advanced:[\s\S]*?AdvancedWorkspaceView\(columnVisibility:\s*columnVisibility\)/.test(files.content)
      && /private func openProjectAttentionTarget\(_ item:\s*AttentionItem\)[\s\S]*?case "provider_profile":[\s\S]*?SettingsNavigation\.openProvider\(\)[\s\S]*?case "app_data":[\s\S]*?SettingsNavigation\.openProviderObservability\(\)/.test(files.content)
      && /func openAdvancedConfiguration\(\)[\s\S]*?configScopeFilter = \.all[\s\S]*?selectAppRoute\(\.advanced\)[\s\S]*?selectDefaultConfigDocumentOrOverview\(\)/.test(files.store)
      && /store\.openAdvancedConfiguration\(\)[\s\S]*?MainWindowCoordinator\.restoreMainWindow\(\)[\s\S]*?settings\.advanced\.open-configuration/.test(files.settings)
      && /AdvancedConfigurationDetailView[\s\S]*?AgentConfigDetailPanel\(\)/.test(files.advancedWorkspace)
      && /AdvancedConfigurationDetailView[\s\S]*?\.navigationTitle\(UIStrings\.appWindowTitle\)/.test(files.advancedWorkspace)
      && /PrivacyPathText\(path:\s*path/.test(files.agentConfigWorkspace)
      && /ConfigContentRedactor\.redactedForDisplay/.test(files.agentConfigWorkspace)
      && /static func openProvider\(\)[\s\S]*?tab:\s*\.provider/.test(files.settingsNavigation)
      && /static func openAdvanced\(\)[\s\S]*?tab:\s*\.advanced/.test(files.settingsNavigation),
  },
  {
    label: "settings AI provider uses signed previews and explicit confirmations without autosave",
    text: files.settings + "\n" + files.store + "\n" + files.confirmedMutationLane,
    passed: !/providerAutosave|submitProviderAutosave|cancelPendingProviderAutosave/.test(
      files.settings + "\n" + files.store + "\n" + files.confirmedMutationLane
    )
      && /await store\.previewDeleteAIProviderSettings\(\)/.test(files.settings)
      && /await store\.previewAIProviderConnectionTest\(\)/.test(files.settings)
      && /await store\.previewSaveAIProviderSettings\(draft:\s*providerDraft\)/.test(files.settings)
      && /if let preview = store\.aiProviderActionPreview,[\s\S]*?let pendingAction = store\.aiProviderPendingAction[\s\S]*?providerActionPreview\(preview,\s*pendingAction:\s*pendingAction\)/.test(files.settings)
      && /private func providerActionPreview\([\s\S]*?preview\.action\.impacts[\s\S]*?preview\.action\.network[\s\S]*?preview\.expectedRevision[\s\S]*?preview\.action\.readback/.test(files.settings)
      && /private func confirmProviderAction\([\s\S]*?confirmSaveAIProviderSettings\(draft:\s*providerDraft\)[\s\S]*?confirmDeleteAIProviderSettings\(\)[\s\S]*?confirmAIProviderConnectionTest\(\)/.test(files.settings)
      && /func confirmSaveAIProviderSettings\([\s\S]*?guard aiProviderPendingAction == \.save,[\s\S]*?let preview = aiProviderActionPreview[\s\S]*?defer \{[\s\S]*?aiProviderActionPreview = nil[\s\S]*?aiProviderPendingAction = nil/.test(files.store)
      && /func confirmDeleteAIProviderSettings\([\s\S]*?guard aiProviderPendingAction == \.delete,[\s\S]*?let preview = aiProviderActionPreview[\s\S]*?defer \{[\s\S]*?aiProviderActionPreview = nil[\s\S]*?aiProviderPendingAction = nil/.test(files.store)
      && /func confirmAIProviderConnectionTest\([\s\S]*?guard aiProviderPendingAction == \.test,[\s\S]*?let preview = aiProviderActionPreview[\s\S]*?defer \{[\s\S]*?aiProviderActionPreview = nil[\s\S]*?aiProviderPendingAction = nil/.test(files.store),
  },
  {
    label: "settings exposes screenshot privacy mode as app-local preference",
    text: files.settings,
    pattern: /@AppStorage\(DisplayText\.screenshotPrivacyModeStorageKey\)[\s\S]*?screenshotPrivacyModeEnabled[\s\S]*?Toggle\(UIStrings\.privacyScreenshotMode,\s*isOn:\s*\$screenshotPrivacyModeEnabled\)/,
  },
  {
    label: "privacy path helper redacts and collapses local paths",
    text: files.formatter,
    pattern: /screenshotPrivacyModeStorageKey[\s\S]*?static func privacyPath[\s\S]*?redactLocalPath[\s\S]*?collapsePath/,
  },
  {
    label: "privacy path view supports explicit reveal",
    text: files.privacyPath,
    pattern: /struct PrivacyPathRow[\s\S]*?@AppStorage\(DisplayText\.screenshotPrivacyModeStorageKey\)[\s\S]*?UIStrings\.privacyRevealPath[\s\S]*?UIStrings\.privacyScreenshotSafe/,
  },
  {
    label: "secondary sidebar project menu uses privacy path display for project paths",
    text: files.sidebar,
    pattern: /private var projectHelp:[\s\S]*?DisplayText\.privacyPath\(rootPath,\s*privacyModeEnabled:\s*true\)/,
  },
  {
    label: "shared surfaces use adaptive native panels",
    text: files.nativePanelSurface,
    passed: /RoundedRectangle\(cornerRadius:\s*CGFloat\(UIOptimizationPresentation\.surfaceCornerRadius\)\)[\s\S]*?\.fill\(Color\.agentCopilotPanelBackground\)/.test(files.nativePanelSurface)
      && /static var agentCopilotPanelBackground:[\s\S]*?Color\(nsColor:\s*\.controlBackgroundColor\)/.test(files.nativePanelSurface)
      && /static var agentCopilotWindowBackground:[\s\S]*?Color\(nsColor:\s*\.windowBackgroundColor\)/.test(files.nativePanelSurface),
  },
  {
    label: "localized LLM action labels are present",
    text: files.localizable,
    pattern: /"llm\.action\.analyze".*"llm\.action\.recommend".*"llm\.action\.explainConflict".*"llm\.action\.draftFrontmatter"/s,
  },
  {
    label: "localized screenshot privacy labels are present",
    text: files.localizable,
    pattern: /"settings\.privacy\.screenshotMode".*"settings\.privacy\.screenshotBoundary".*"privacy\.path\.reveal".*"privacy\.path\.screenshotSafe"/s,
  },
  {
    label: "localized task cockpit labels are present",
    text: files.localizable,
    passed: [
      "taskCockpit.boundary",
      "taskCockpit.action.build",
      "taskCockpit.empty.result",
      "taskCockpit.recommendedSkill",
    ].every((key) => files.localizable.includes(`"${key}" =`)),
  },
  {
    label: "localized skill manager workflow and unavailable-tool labels are present",
    text: files.localizable,
    pattern: /"skillManager\.workflow\.accessibility".*"skillManager\.workflow\.searchInstall".*"skillManager\.workflow\.installedUpdates".*"skillManager\.toolUnavailable\.title".*"skillManager\.toolUnavailable\.message".*"skillManager\.inventory".*"skillManager\.chooseZip"/s,
  },
  {
    label: "localized remediation and permissions labels are present",
    text: files.localizable,
    pattern: /"findings\.remediation".*"permissions\.undeclared".*"permissions\.declarationNote"/s,
  },
];

const detailEvidenceLists = [
  "TaskCockpitEvidenceList",
];

const nativeIPCCleanupChecks = [
  {
    label: "ServiceClient keeps short-lived stdio Process IPC shape",
    passed: /Process\(\)/.test(files.serviceIPC)
      && /\.standardInput\s*=\s*stdin/.test(files.serviceIPC)
      && /\.standardOutput\s*=\s*stdout/.test(files.serviceIPC)
      && /\.standardError\s*=\s*stderr/.test(files.serviceIPC),
  },
  {
    label: "ServiceClient wraps runService with task cancellation cleanup",
    passed: /processRunner\.run\(\s*executableURL:\s*resolveServiceURL\(\),\s*input:\s*input(?:,\s*timeoutNanoseconds:\s*timeoutNanoseconds)?\s*\)/.test(runServiceBody)
      && /withTaskCancellationHandler/.test(files.serviceProcessRunner),
  },
  {
    label: "ServiceClient terminates and reaps the child process on cancel or timeout",
    passed: /terminate\s*\(/.test(files.serviceProcessRunner)
      && /waitUntilExit\s*\(/.test(files.serviceProcessRunner)
      && /(onCancel|Task\.isCancelled|Cancellation|cancel|timeout|timedOut|forceTerminate)/i.test(files.serviceProcessRunner),
  },
  {
    label: "ServiceClient closes stdin and releases stdout and stderr handles during IPC cleanup",
    passed: countMatches(files.serviceIPC, /fileHandleForWriting[\s\S]{0,180}\.(?:close|closeFile)\s*\(/g) >= 1
      && /stdinWriter\?\.(?:close|closeFile)\s*\(/.test(files.serviceProcessRunner)
      && /stdoutReader\s*=\s*nil/.test(files.serviceProcessRunner)
      && /stderrReader\s*=\s*nil/.test(files.serviceProcessRunner),
  },
  {
    label: "ServiceClient clears pipe readability handlers or releases read handles",
    passed: /readabilityHandler\s*=\s*nil/.test(files.serviceIPC)
      || (/stdoutReader\s*=\s*nil/.test(files.serviceProcessRunner)
        && /stderrReader\s*=\s*nil/.test(files.serviceProcessRunner)),
  },
  {
    label: "ServiceClient protects continuations from stale or duplicate completion",
    passed: /(resumeOnce|finishOnce|completeOnce|didResume|hasResumed|isCompleted|completed|finished|cleanedUp|stale)/i.test(files.serviceIPC)
      && /(NSLock|DispatchQueue|ManagedAtomic|lock\s*\(|actor\b)/.test(files.serviceIPC)
      && /(if\s+cleanedUp|guard\s+!.*cleanedUp|markCancelled|Task\.checkCancellation)/s.test(files.serviceProcessRunner),
  },
  {
    label: "ServiceClient does not introduce a daemon, socket, XPC, or network redesign",
    passed: !/(^|\n)\s*import\s+Network\b|NWListener|NWConnection|NSXPCConnection|URLSessionWebSocketTask|SocketPort|UnixDomainSocket|\bdaemon\b|\blaunchd\b/.test(files.serviceIPC),
  },
  {
    label: "ServiceRequest IPC payload remains id, method, and params only",
    passed: /let\s+id:\s*String/.test(serviceRequestBody)
      && /let\s+method:\s*String/.test(serviceRequestBody)
      && /let\s+params:\s*Params/.test(serviceRequestBody)
      && !/(cancel|timeout|pid|socket|daemon|token)/i.test(serviceRequestBody),
  },
  {
    label: "Agent Copilot protocol method surface matches the supported method contract",
    passed: supportedMethods.length > 0
      && JSON.stringify(supportedMethods) === JSON.stringify(statusFixtureMethods),
  },
  {
    label: "protocol surface has no IPC control, daemon, process, or socket methods",
    passed: forbiddenProtocolMethods.length === 0,
  },
];

const customChecks = [
  {
    label: "sidebar omits the retired Work section",
    passed: !/Section\(UIStrings\.text\("nav\.work",\s*"Work"\)\)/.test(files.sidebar)
      && !/SidebarWorkSurfaceRow/.test(files.sidebar)
      && !/ForEach\(DetailSection\.primaryWorkCases\)/.test(files.sidebar),
  },
  {
    label: "advanced configuration rows use muted native selection",
    passed: ["ConfigCurrentDocumentSidebarRow", "ConfigSnapshotSidebarRow"].every((name) => {
      const body = extractStructBody(files.sidebar, name);
      return body.includes(".optimizedSidebarSelection(isSelected: isSelected)")
        && !/foregroundStyle\(isSelected \? (?:Color\.)?\.?white/.test(body)
        && !/fill\(Color\.accentColor\)/.test(body);
    })
      && /struct SidebarSelectionPresentation:[\s\S]*?usesSaturatedAccentBackground = false[\s\S]*?usesWhiteSelectedText = false[\s\S]*?accentLineWidth = 3/.test(files.uiOptimization)
      && /private struct OptimizedSidebarSelectionModifier:[\s\S]*?selectedContentBackgroundColor[\s\S]*?UIOptimizationPresentation\.sidebarSelection\.accentLineWidth/.test(files.sidebar)
      && !/ListPageCardBackgroundModifier|SessionSidebarRow|SkillRow/.test(files.sidebar),
  },
  {
    label: "advanced configuration sidebar uses compact search and icon refresh controls",
    passed: /struct SidebarSecondaryListPresentation:[\s\S]*?minimumSearchWidth = 220[\s\S]*?refreshUsesIconOnly = true/.test(files.uiOptimization)
      && /private struct SidebarSearchField:[\s\S]*?TextField\(placeholder,\s*text:\s*\$text\)[\s\S]*?\.textFieldStyle\(\.roundedBorder\)[\s\S]*?\.controlSize\(\.small\)[\s\S]*?\.frame\(minWidth:\s*minimumWidth,\s*maxWidth:\s*\.infinity\)/.test(files.sidebar)
      && /private var configToolbar:[\s\S]*?VStack\(alignment:\s*\.leading,\s*spacing:\s*8\)[\s\S]*?HStack\(alignment:\s*\.center,\s*spacing:\s*CGFloat\(layout\.filterControlSpacing\)\)[\s\S]*?configScopePicker[\s\S]*?configRefreshButton\([\s\S]*?configSearchField/.test(files.sidebar)
      && /private func configRefreshButton\(width:\s*CGFloat,\s*height:\s*CGFloat\)[\s\S]*?Image\(systemName:\s*"arrow\.clockwise"\)[\s\S]*?\.accessibilityLabel\(UIStrings\.reload\)/.test(files.sidebar)
      && !/sessionToolbar|sessionRefreshButton|SessionScopeToggle/.test(files.sidebar),
  },
  {
    label: "settings owns background-loaded lightweight Provider Observability dashboard",
    passed: /ProviderObservabilitySettingsPanel\(\)/.test(files.settings)
      && /loadAIProviderStatusIfNeeded\(\)/.test(files.settings)
      && !/store\.reload\(\)/.test(files.settings)
      && /case \.providerObservability:\s*break/.test(files.settings)
      && /func loadAIProviderStatusIfNeeded\(\) async/.test(files.store)
      && /func loadProviderObservabilityIfNeeded\(\) async/.test(files.store)
      && /scheduleStartupSupplementalLoads\([\s\S]*?forceProviderObservability:\s*false/.test(files.store)
      && /scheduleReloadSupplementalLoads\([\s\S]*?forceProviderObservability:\s*true/.test(files.store)
      && /loadProviderObservabilityDuringRefresh\(force:\s*forceProviderObservability\)/.test(files.store)
      && /allowDuringRefresh:\s*Bool/.test(files.store)
      && /providerObservabilityRowLimit\s*=\s*100/.test(files.store)
      && /ProviderObservabilityDateRangeControls/.test(files.providerObservabilitySettings)
      && /providerObservabilityDateRange/.test(files.uiStrings + "\n" + files.localizable + "\n" + files.localizableZh)
      && /includeBudgetHints:\s*false/.test(files.store)
      && /includeRetentionRecommendations:\s*false/.test(files.store)
      && /includeEvidence:\s*false/.test(files.store)
      && !/\.task\s*\{[\s\S]{0,240}?loadProviderObservability\(/.test(files.providerObservabilitySettings)
      && /case \.providerObservability:[\s\S]*?return UIStrings\.providerObservabilityTitle/.test(files.settings)
      && /case \.providerObservability:[\s\S]*?return "waveform\.path\.ecg\.rectangle"/.test(files.settings)
      && /case \.providerObservability:\s*ProviderObservabilitySettingsPanel\(\)/.test(files.settings)
      && /ScrollView\s*\{\s*VStack\(alignment:\s*\.leading,\s*spacing:\s*14\)/.test(files.providerObservabilitySettings)
      && /ProviderObservabilitySummaryStrip\(metrics:\s*summaryMetrics\)/.test(files.providerObservabilitySettings)
      && /ProviderObservabilitySettingsChartsPanel\(result:\s*result\)/.test(files.providerObservabilitySettings)
      && /ProviderObservabilityLoadingCard\(isLoading:\s*store\.isLoadingProviderObservability\)/.test(files.providerObservabilitySettings)
      && /providerObservabilityAutoLoadsAtStartup\s*=\s*true/.test(files.uiOptimization)
      && /providerObservabilityHasLocalBuildAction\s*=\s*false/.test(files.uiOptimization)
      && /providerObservabilityHidesRawLogList\s*=\s*true/.test(files.uiOptimization)
      && /providerObservabilitySummaryMetricCount\s*=\s*5/.test(files.uiOptimization)
      && /providerObservabilityChartRowLimit\s*=\s*5/.test(files.uiOptimization)
      && /providerObservabilityUsesScopedScroll\s*=\s*true/.test(files.uiOptimization)
      && /providerObservabilityDisablesSelectionOverlay\s*=\s*true/.test(files.uiOptimization)
      && /providerObservabilityAvoidsAdaptiveGrids\s*=\s*true/.test(files.uiOptimization)
      && !/ProviderObservabilitySettingsMode|ProviderObservabilityLogSettingsView|statusFilter|providerFilter|modelFilter|destinationFilter|showIssuesOnly|searchText|providerObservabilityAction/.test(files.providerObservabilitySettings)
      && !/\.textSelection\(\.enabled\)/.test(files.providerObservabilitySettings)
      && !/LazyVGrid|\bGrid\(|GeometryReader|PrivacyEvidenceText|ProviderObservabilityChartsPanel|DetailMetricGrid|CompactMetadataGrid/.test(files.providerObservabilitySettings)
      && /providerObservability\.successRate/.test(files.uiStrings + "\n" + files.localizable + "\n" + files.localizableZh)
      && /providerObservability\.empty\.dashboardTitle/.test(files.providerObservabilitySettings + "\n" + files.localizable),
  },
  {
    label: "Agent Config moved from Settings into the main sidebar workflow",
    passed: !/AgentConfigSettingsPanel\(/.test(files.settings)
      && /SidebarContentMode\.config/.test(files.sidebar)
      && /AdvancedConfigurationDetailView\(\)/.test(files.advancedWorkspace)
      && /AgentConfigDetailPanel\(\)/.test(files.advancedWorkspace)
      && /struct AgentConfigOverviewDetailPanel/.test(files.agentConfigWorkspace)
      && /struct AgentConfigSnapshotDetailPanel/.test(files.agentConfigWorkspace)
      && !/private struct AgentConfigSnapshotDetailPanel[\s\S]*?DetailMetricGrid[\s\S]*?SummaryChip\(title:\s*UIStrings\.agent/.test(files.agentConfigWorkspace),
  },
  {
    label: "Agent Workspace does not expose the retired evidence surface navigation grid",
    passed: !/AgentProfileNavigationGrid|agentCopilot\.evidenceSurfaces|selectedSidebarSelection\s*=\s*\.work\(section\)/.test(files.sessionWorkspaceDetail + "\n" + files.sidebar),
  },
  {
    label: "modal workflows share liquid-glass sheet chrome, columns, and inline feedback",
    passed: /static let workflowSheet = WorkflowSheetPresentation\(\)/.test(files.uiOptimization)
      && /struct WorkflowSheetPresentation:[\s\S]*?titlebarStyle = WorkflowSheetTitlebarStyle\.liquidGlass[\s\S]*?closeActionPlacement = WorkflowSheetCloseActionPlacement\.trailingTitlebar[\s\S]*?feedbackStyle = WorkflowSheetFeedbackStyle\.inlineTintedBanner[\s\S]*?columnLayout = WorkflowSheetColumnLayout\.twoColumn/.test(files.uiOptimization)
      && /struct WorkflowSheetShell<Content: View>:[\s\S]*?Label\(title,\s*systemImage:\s*systemImage\)[\s\S]*?Button\s*\{[\s\S]*?dismiss\(\)[\s\S]*?\} label:\s*\{[\s\S]*?Label\(UIStrings\.done,\s*systemImage:\s*"xmark"\)[\s\S]*?\.background\(\.bar\)/.test(files.workflowSheet)
      && /struct WorkflowSheetSplitLayout<Primary: View,\s*Secondary: View>:[\s\S]*?primary\(\)[\s\S]*?Divider\(\)[\s\S]*?secondary\(\)/.test(files.workflowSheet)
      && /struct WorkflowSheetInlineBanner:[\s\S]*?Label\(message,\s*systemImage:\s*style\.systemImage\)[\s\S]*?\.background\(style\.color\.opacity\(0\.08\)[\s\S]*?Rectangle\(\)[\s\S]*?\.fill\(style\.color\)/.test(files.workflowSheet)
      && /private struct SkillPackageManagerSheet:[\s\S]*?WorkflowSheetShell\([\s\S]*?SkillManagerPanel\([\s\S]*?showsHeader:\s*false,[\s\S]*?entryContext:\s*entryContext/.test(files.skillsWorkspace)
      && /struct SkillManagerPanel:[\s\S]*?WorkflowSheetSplitLayout\(primaryMinWidth:\s*430,\s*secondaryWidth:\s*380\)[\s\S]*?workflowPicker[\s\S]*?searchSection[\s\S]*?inventorySection[\s\S]*?actionSection[\s\S]*?previewSection/.test(files.skillManager)
      && !/struct SkillPackageManagerSheet:[\s\S]*?ErrorBanner\(message:\s*error\)/.test(files.skillsWorkspace)
      && !/struct SkillPackageManagerSheet:[\s\S]*?SuccessBanner\(message:\s*message\)/.test(files.skillsWorkspace),
  },
  {
    label: "task readiness is inline in Project Overview and preserves task_cockpit compatibility",
    passed: /private var overviewSections:[\s\S]*?ProjectStatusSection\([\s\S]*?taskReadinessSection[\s\S]*?ProjectAttentionSection\([\s\S]*?ProjectContinueWorkSection\(/.test(files.projectOverview)
      && /private var taskReadinessSection:[\s\S]*?TaskCockpitPanel\([\s\S]*?providerGateMessage:\s*taskProviderGateMessage/.test(files.projectOverview)
      && /private var taskProviderGateMessage:[\s\S]*?status\.serviceAvailable[\s\S]*?status\.enabled/.test(files.projectOverview)
      && /task_cockpit/.test(files.serviceProtocol)
      && !/taskCockpitHistory|recordTaskCockpitHistory/.test(files.taskCockpit + "\n" + files.store)
      && !extractStructBody(files.sidebar, "SidebarView").includes("TaskPreflightPreviewSheet")
      && !/case preflight/.test(files.sidebarSelection)
      && !/selectedSidebarSelection\s*=\s*\.preflight/.test(files.sidebar + "\n" + files.storeSurface),
  },
  {
    label: "legacy private-content cleanup is persistent, reviewable, and explicitly confirmed",
    text: [
      files.content,
      files.legacyPrivateContentBanner,
      files.legacyPrivateContentCard,
      files.providerObservabilitySettings,
      files.settingsNavigation,
      files.settings,
      files.storeLegacyPrivacy,
    ].join("\n"),
    passed: /LegacyPrivateContentGlobalBanner\(\)[\s\S]*?navigationShell/.test(files.content)
      && /legacyPrivateContentInspection\?\.cleanupRequired == true[\s\S]*?legacyPrivateContentCleanupError != nil/.test(files.legacyPrivateContentBanner)
      && /Button\(UIStrings\.legacyPrivateContentOpenSettings\)[\s\S]*?SettingsNavigation\.openProviderObservability\(\)/.test(files.legacyPrivateContentBanner)
      && /static func openProviderObservability\(\)[\s\S]*?tab:\s*\.providerObservability[\s\S]*?notification:\s*providerObservabilityRequested[\s\S]*?private static func open\(tab:\s*SettingsTab[\s\S]*?UserDefaults\.standard\.set\([\s\S]*?tab\.rawValue[\s\S]*?showSettingsWindow:/.test(files.settingsNavigation)
      && /LegacyPrivateContentCleanupCard\(\)/.test(files.providerObservabilitySettings)
      && /previewLegacyPrivateContentCleanup\(\)[\s\S]*?confirmLegacyPrivateContentCleanup\(\)/.test(files.legacyPrivateContentCard)
      && /cleanupLegacyPrivateContent\(preview:\s*preview\)/.test(files.storeLegacyPrivacy)
      && /inspectLegacyPrivateContent\(\)[\s\S]*?previewLegacyPrivateContentCleanup\(\)[\s\S]*?confirmLegacyPrivateContentCleanup\(\)/.test(files.storeLegacyPrivacy)
      && !/privacy\.cleanupLegacyContent/.test(files.content),
  },
  {
    label: "settings provider status copy uses precise disabled and audit-state labels",
    passed: /static var aiProviderDisabledReason:[\s\S]*?settings\.aiProvider\.disabledReason/.test(files.uiStrings)
      && /static var aiProviderAuditApplied:[\s\S]*?settings\.aiProvider\.audit\.applied/.test(files.uiStrings)
      && /static var aiProviderAuditNotApplied:[\s\S]*?settings\.aiProvider\.audit\.notApplied/.test(files.uiStrings)
      && /static var aiProviderAuditStored:[\s\S]*?settings\.aiProvider\.audit\.stored/.test(files.uiStrings)
      && /static var aiProviderAuditNotStored:[\s\S]*?settings\.aiProvider\.audit\.notStored/.test(files.uiStrings)
      && /SettingsMetadataRow\(label:\s*UIStrings\.aiProviderDisabledReason,\s*value:\s*UIStrings\.localizedServiceMessage\(disabledReason\)\)/.test(files.settings)
      && /audit\.redactionApplied \? UIStrings\.aiProviderAuditApplied : UIStrings\.aiProviderAuditNotApplied/.test(files.settings)
      && /audit\.promptStored \? UIStrings\.aiProviderAuditStored : UIStrings\.aiProviderAuditNotStored/.test(files.settings)
      && /audit\.responseStored \? UIStrings\.aiProviderAuditStored : UIStrings\.aiProviderAuditNotStored/.test(files.settings)
      && /"settings\.aiProvider\.disabledReason" = "Disabled reason";/.test(files.localizable)
      && /"settings\.aiProvider\.audit\.notStored" = "Not stored";/.test(files.localizable)
      && /"settings\.aiProvider\.disabledReason" = "禁用原因";/.test(files.localizableZh)
      && /"settings\.aiProvider\.audit\.notStored" = "未存储";/.test(files.localizableZh)
      && !/SettingsMetadataRow\(label:\s*UIStrings\.aiProviderUnconfigured,\s*value:\s*UIStrings\.localizedServiceMessage\(disabledReason\)\)/.test(files.settings)
      && !/aiProviderAuditPromptStored,\s*value:\s*audit\.promptStored \? UIStrings\.llmEnabled : UIStrings\.llmDisabled/.test(files.settings),
  },
  {
    label: "task cockpit presentation uses structured tokens instead of English substring classification",
    passed: /private var hasRouteAmbiguity:[\s\S]*?candidateScores[\s\S]*?routeCandidateCount/.test(files.taskCockpit)
      && /enum TaskCockpitSignalClassifier[\s\S]*?static func classification\(for row: TaskCockpitContextRow\)[\s\S]*?signalTokens\(for:\s*row\)/.test(files.taskCockpitModel)
      && /enum TaskCockpitSignalClassifier[\s\S]*?private static func signalTokens\(for row: TaskCockpitContextRow\) -> Set<String>/.test(files.taskCockpitModel)
      && /enum TaskCockpitSignalClassifier[\s\S]*?static func normalizedToken\(_ value: String\) -> String/.test(files.taskCockpitModel)
      && /private static func isReviewOnlyRisk\(_ row: TaskCockpitContextRow\) -> Bool[\s\S]*?TaskCockpitSignalClassifier\.classification\(for:\s*row\)/.test(files.taskCockpit)
      && /private static func isInternalBoundary\(_ row: TaskCockpitContextRow\) -> Bool[\s\S]*?TaskCockpitSignalClassifier\.classification\(for:\s*row\)/.test(files.taskCockpit)
      && /private static func isReviewOnlyRisk\(_ row: TaskCockpitContextRow\) -> Bool[\s\S]*?TaskCockpitSignalClassifier\.classification\(for:\s*row\)/.test(files.taskCockpitModel)
      && /private static func isInternalBoundary\(_ row: TaskCockpitContextRow\) -> Bool[\s\S]*?TaskCockpitSignalClassifier\.classification\(for:\s*row\)/.test(files.taskCockpitModel)
      && !/normalized\.contains\("task readiness is blocked"\)/.test(files.taskCockpit + files.taskCockpitModel)
      && !/normalized\.contains\("routing confidence is blocked"\)/.test(files.taskCockpit + files.taskCockpitModel)
      && !/normalized\.contains\("small score margin"\)/.test(files.taskCockpit + files.taskCockpitModel)
      && !/normalized\.contains\("close or overlapping alternatives"\)/.test(files.taskCockpit + files.taskCockpitModel)
      && !/normalized\.contains\("read-only"\)/.test(files.taskCockpit + files.taskCockpitModel)
      && !/normalized\.contains\("provider not sent"\)/.test(files.taskCockpit + files.taskCockpitModel)
      && !/normalized\.contains\("task cockpit combined"\)/.test(files.taskCockpit + files.taskCockpitModel),
  },
  {
    label: "skill workspace filter controls expose their roles and stable accessibility identifiers",
    passed: /private var filterControls:[\s\S]*?Picker\([\s\S]*?skills\.workspace\.view\.label[\s\S]*?accessibilityIdentifier\("skills\.workspace\.view"\)/.test(files.skillsWorkspaceList)
      && /TextField\([\s\S]*?skills\.workspace\.search\.placeholder[\s\S]*?accessibilityIdentifier\("skills\.workspace\.search"\)/.test(files.skillsWorkspaceList)
      && /Picker\(UIStrings\.agent,[\s\S]*?accessibilityIdentifier\("skills\.workspace\.agent"\)/.test(files.skillsWorkspaceList)
      && /Picker\(UIStrings\.sort,[\s\S]*?accessibilityIdentifier\("skills\.workspace\.sort"\)/.test(files.skillsWorkspaceList)
      && /accessibilityIdentifier\("skills\.workspace\.sort-direction"\)[\s\S]*?accessibilityLabel\(workspace\.sortDirection\.title\)/.test(files.skillsWorkspaceList),
  },
  {
    label: "native panel surface uses shared white presentation corner radius",
    passed: /static let surfaceCornerRadius = sidebarSelection\.rowCornerRadius/.test(files.uiOptimization)
      && /RoundedRectangle\(cornerRadius:\s*CGFloat\(UIOptimizationPresentation\.surfaceCornerRadius\)\)/.test(files.nativePanelSurface)
      && !/RoundedRectangle\(cornerRadius:\s*8\)/.test(files.nativePanelSurface),
  },
  {
    label: "Skill Manager uses a skill-first inventory with action-time targeting",
    passed: /enum SkillManagerWorkflow:[\s\S]*?case searchInstall[\s\S]*?case installedUpdates/.test(files.skillManagerModel)
      && !/case localLibrary/.test(files.skillManagerModel)
      && /@State private var selectedWorkflow:\s*SkillManagerWorkflow = \.searchInstall/.test(files.skillManager)
      && /@State private var selectedSkill:\s*SkillManagerSelection\?/.test(files.skillManager)
      && /private var searchSection:[\s\S]*?skillManagerSearchQuery[\s\S]*?SkillManagerSelectableRow/.test(files.skillManager)
      && /private var inventorySection:[\s\S]*?Picker\(UIStrings\.scope,\s*selection:\s*\$store\.skillManagerScope\)[\s\S]*?skillManagerInventoryItems/.test(files.skillManager)
      && /private var actionSection:[\s\S]*?if let selectedSkill[\s\S]*?actionPicker[\s\S]*?actionOptions[\s\S]*?actionButton/.test(files.skillManager)
      && /private func agentPicker\(available:[\s\S]*?actionAgentIDs[\s\S]*?LazyVGrid/.test(files.skillManager)
      && /private var feedback:[\s\S]*?WorkflowSheetInlineBanner\(message:\s*error,\s*style:\s*\.error\)[\s\S]*?WorkflowSheetInlineBanner\(message:\s*message,\s*style:\s*\.success\)/.test(files.skillManager)
      && /externalMutationDisabled/.test(files.skillManager)
      && /externalManagerUnavailableMessage/.test(files.skillManager)
      && /store\.skillManagerErrorMessage/.test(files.skillManager)
      && /store\.skillManagerMessage/.test(files.skillManager)
      && /clearSkillManagerWorkflowPreviews/.test(files.store + "\n" + files.skillManager + "\n" + files.sidebar)
      && /store\.skillManagerMutationConfirmation/.test(files.skillManager)
      && /await store\.applySkillManagerLocalArchiveUpdate\(confirmation:\s*value\)/.test(files.skillManager)
      && /await store\.refreshSkillManagerData\(\)/.test(files.skillManager)
      && /SkillManagerInventoryBuilder\.build\([\s\S]*?installed:\s*skillManagerInstalled\?\.installed \?\? \[\][\s\S]*?catalogSkills:\s*skills[\s\S]*?localLibrarySkills:\s*localSkillLibrarySkills/.test(files.storeDerivedState)
      && /enum SkillManagerInventoryBuilder[\s\S]*?deduplicatedInstalled\(installed\)[\s\S]*?consumedSourcePaths[\s\S]*?editableCatalogSources[\s\S]*?sharedAgentsSourceDirectory/.test(files.skillManagerModel)
      && /\.onChange\(of:\s*store\.skillManagerInventoryItems\)[\s\S]*?items\.first\(where:\s*\{ \$0\.id == selectedItem\.id \}\)/.test(files.skillManager)
      && !/\.task\s*\{[\s\S]*?listSkillManagerInstalled/.test(files.skillManager)
      && !/\$store\.skillManagerOwner|\$store\.skillManagerSource|\$store\.skillManagerInstallSkillName|\$store\.skillManagerRemoveSkillName/.test(files.skillManager)
      && !/Toggle\(UIStrings\.text\("skillManager\.network"/.test(files.skillManager)
      && !/skillManagerDistribution/.test(files.skillManager),
  },
  {
    label: "Skill Manager search is keyword-only and result-driven",
    passed: /private var searchSection/.test(files.skillManager)
      && /skillManagerSearchQuery/.test(files.skillManager)
      && /store\.searchSkillManager\(\)/.test(files.skillManager)
      && /SkillManagerSelectableRow/.test(files.skillManager)
      && /result\.results\.isEmpty[\s\S]*?skillManager\.search\.noResults/.test(files.skillManager)
      && /status\.canLoadMore[\s\S]*?loadMoreSkillManagerSearchResults[\s\S]*?showAllReturnedSkillManagerSearchResults/.test(files.skillManager)
      && !/skillManagerOwner|skillManagerSuggestion/.test(files.skillManager),
  },
  {
    label: "Skill Manager exposes local ZIP import and source-aware local update",
    passed: /skill-manager\.local-import\.choose/.test(files.skillManager)
      && /private func handleImportArchiveSelection[\s\S]*?previewSkillManagerLocalArchiveImport\(archivePath:\s*url\.path\)/.test(files.skillManager)
      && /private func inventorySourceBadge[\s\S]*?source\.manager[\s\S]*?source\.localProject[\s\S]*?source\.localGlobal[\s\S]*?source\.localLibrary[\s\S]*?source\.localExternal/.test(files.skillManager)
      && /if item\.origin == \.local[\s\S]*?skillManager\.localUpdate\.help/.test(files.skillManager)
      && /if selection\.isLocal[\s\S]*?isChoosingArchive = true[\s\S]*?previewSkillManagerUpdate/.test(files.skillManager)
      && /enum SkillManagerInventoryActionPolicy[\s\S]*?if item\.origin == \.local, item\.localInstanceID == nil\s*\{\s*return \[\.remove\]/.test(files.skillManagerModel)
      && /skillManagerLocalArchiveImportConfirmation[\s\S]*?localArchiveImport\(confirmation\)/.test(files.skillManager)
      && /sourceKind:\s*String\?/.test(files.skillManagerModel)
      && /let path:\s*String\?/.test(files.skillManagerModel)
      && /sharedAgentsPathSuffix\(record\.path \?\? record\.source\)/.test(files.skillManagerModel)
      && /guard !consumedSourcePaths\.contains\(source\.path\)/.test(files.skillManagerModel)
      && /case project[\s\S]*?case global/.test(files.skillManagerModel),
  },
  {
    label: "Skill Manager target controls appear only in install and remove actions",
    passed: /private func actionOptions\(for selection:[\s\S]*?case \.install:[\s\S]*?agentPicker[\s\S]*?case \.remove:[\s\S]*?agentPicker/.test(files.skillManager)
      && /case \.update:[\s\S]*?skillManager\.update\.affected/.test(files.skillManager)
      && !/targetControls/.test(files.skillManager),
  },
  {
    label: "Skill Manager removes search suggestions and manual package identity fields",
    passed: !/skillManagerSearchSuggestions|skillManagerSuggestionBar|SkillManagerSuggestionPill/.test(files.skillManager + "\n" + files.skillManagerModel)
      && !/skillManagerOwner|skillManagerSource|skillManagerInstallSkillName|skillManagerRemoveSkillName/.test(files.skillManager),
  },
  {
    label: "retired smart analysis detail copy is removed from current UI resources",
    passed: !/Use focused smart analysis panels for quality scoring, task fit, and routing\./.test(files.detailSection)
      && !/"detail\.section\.analysis\.summary"/.test(files.localizable)
      && !/"detail\.analysisReview"/.test(files.localizable),
  },
  {
    label: "guarded config changes live behind the selected skill aggregate",
    passed: !/SafeBatchTogglePanel|BatchTogglePreviewSummary|BatchSkillOperationSheet/.test(files.sidebar)
      && /struct SkillsWorkspaceView:[\s\S]*?\.sheet\(isPresented:\s*\$isConfigOperationPresented\)[\s\S]*?BatchSkillOperationSheet\(\)/.test(files.skillsWorkspace)
      && /private func openConfigFlow\([\s\S]*?prepareSkillTogglePreview\([\s\S]*?instanceIDs:\s*aggregate\.instanceIDs[\s\S]*?isConfigOperationPresented = true/.test(files.skillsWorkspace)
      && /Toggle\(isOn:\s*selectionBinding\)/.test(files.batchSkillOperation)
      && /store\.selectAllVisibleBatchToggleSkills\(\)/.test(files.batchSkillOperation)
      && /store\.clearBatchToggleSelection\(\)/.test(files.batchSkillOperation)
      && /await store\.previewVisibleBatchToggle\(\)/.test(files.batchSkillOperation)
      && /await store\.applyVisibleBatchTogglePreview\(confirmingPreviewID:\s*previewID\)/.test(files.batchSkillOperation),
  },
  {
    label: "Provider Observability settings presents summary and charts without detailed evidence rows",
    passed: /ProviderObservabilitySummaryStrip\(metrics:\s*summaryMetrics\)/.test(files.providerObservabilitySettings)
      && /ProviderObservabilitySettingsChartsPanel\(result:\s*result\)/.test(files.providerObservabilitySettings)
      && /struct ProviderObservabilitySettingsChartCard/.test(files.providerObservabilitySettings)
      && /providerObservabilityChartModelTokens/.test(files.providerObservabilitySettings)
      && /providerObservabilityChartDestinationCost/.test(files.providerObservabilitySettings)
      && /providerObservabilityChartModelTaskConfidence/.test(files.providerObservabilitySettings)
      && !/ProviderObservabilitySettingsDimensionGroup|ProviderObservabilitySettingsHintGroup|ProviderObservabilitySettingsModelTaskHistoryList|ProviderObservabilitySettingsCallRow|ProviderObservabilitySettingsEvidenceText/.test(files.providerObservabilitySettings),
  },
  {
    label: "sidebar and retired agent profile omit adapter capability content",
    passed: !/SidebarAgentStatusPanel|AdapterCapabilityCard|RefreshStatusView/.test(files.sidebar)
      && !/AgentCapabilitySummaryCard/.test(files.agentSessionDetail)
      && !/capabilityReminders\(from:\s*store\.selectedAdapterCapability\)/.test(files.agentSessionDetail)
      && !/AgentProfileInfoRow/.test(files.agentSessionDetail),
  },
  {
    label: "detail evidence lists are compact, fully expandable, and use privacy rendering",
    passed: detailEvidenceLists.every((name) => {
      const body = extractStructBody(files.detailSurface, name);
      return (body.includes("ForEach(evidence.prefix(")
          || body.includes("DenseDisclosureList(evidence, visibleLimit:")
          || body.includes("ExpandableSummaryList("))
        && body.includes("PrivacyEvidenceText(value: item.detail")
        && body.includes("PrivacyEvidenceText(value: source");
    }),
  },
  ...nativeIPCCleanupChecks,
];

const failures = [
  ...checks.filter((check) => {
    if (check.pattern) {
      return !check.pattern.test(check.text);
    }
    return !check.passed;
  }),
  ...customChecks.filter((check) => !check.passed),
];
if (failures.length > 0) {
  for (const failure of failures) {
    console.error(`native-ui-layout-check: missing ${failure.label}`);
  }
  process.exit(1);
}

console.log(`native-ui-layout-check: ${checks.length + customChecks.length} checks passed`);

async function read(path) {
  return readFile(join(repoRoot, path), "utf8");
}

async function exists(path) {
  try {
    await readFile(join(repoRoot, path));
    return true;
  } catch (error) {
    if (error?.code === "ENOENT") return false;
    throw error;
  }
}

function extractStructBody(text, structName) {
  const marker = `struct ${structName}:`;
  const start = text.indexOf(marker);
  if (start === -1) {
    return "";
  }

  const openBrace = text.indexOf("{", start);
  if (openBrace === -1) {
    return "";
  }

  let depth = 0;
  for (let index = openBrace; index < text.length; index += 1) {
    const char = text[index];
    if (char === "{") {
      depth += 1;
    } else if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        return text.slice(openBrace + 1, index);
      }
    }
  }

  return "";
}

function extractFunctionBody(text, functionName) {
  const match = new RegExp(`func\\s+${escapeRegex(functionName)}\\b`).exec(text);
  if (!match) {
    return "";
  }
  return extractBalancedBody(text, match.index);
}

function extractServiceRequestBody(text) {
  const marker = "struct ServiceRequest";
  const start = text.indexOf(marker);
  if (start === -1) {
    return "";
  }
  return extractBalancedBody(text, start);
}

function extractBalancedBody(text, start) {
  const openBrace = text.indexOf("{", start);
  if (openBrace === -1) {
    return "";
  }

  let depth = 0;
  for (let index = openBrace; index < text.length; index += 1) {
    const char = text[index];
    if (char === "{") {
      depth += 1;
    } else if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        return text.slice(openBrace + 1, index);
      }
    }
  }

  return "";
}

function parseSupportedMethods(rustSource) {
  const block = rustSource.match(/const\s+SUPPORTED_METHODS\s*:\s*&\s*\[\s*&str\s*\]\s*=\s*&\s*\[([\s\S]*?)\];/);
  if (!block) {
    return [];
  }
  return uniqueSorted([...block[1].matchAll(/"([A-Za-z][A-Za-z0-9]*\.[A-Za-z][A-Za-z0-9]*)"/g)].map((match) => match[1]));
}

function parseStatusFixtureMethods(text) {
  try {
    const fixture = JSON.parse(text);
    const methods = fixture?.result?.supported_methods;
    return Array.isArray(methods) ? uniqueSorted(methods.filter((method) => typeof method === "string")) : [];
  } catch {
    return [];
  }
}

function uniqueSorted(values) {
  return [...new Set(values)].sort((left, right) => left.localeCompare(right));
}

function countMatches(text, pattern) {
  return [...text.matchAll(pattern)].length;
}

function escapeRegex(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
