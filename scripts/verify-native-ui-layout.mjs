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
  detail: await read("apps/macos/Sources/SkillsCopilot/Views/DetailView.swift"),
  agentSessionDetail: await read("apps/macos/Sources/SkillsCopilot/Views/AgentSessionDetailPanel.swift"),
  detailSection: await read("apps/macos/Sources/SkillsCopilot/Models/DetailSection.swift"),
  detailOverview: await read("apps/macos/Sources/SkillsCopilot/Views/DetailOverviewSection.swift"),
  detailPrimitives: await read("apps/macos/Sources/SkillsCopilot/Views/DetailPresentationPrimitives.swift"),
  providerObservabilitySettings: await read("apps/macos/Sources/SkillsCopilot/Views/ProviderObservabilitySettingsPanel.swift"),
  skillManager: await read("apps/macos/Sources/SkillsCopilot/Views/SkillManagerPanel.swift"),
  workflowSheet: await read("apps/macos/Sources/SkillsCopilot/Views/WorkflowSheetChrome.swift"),
  skillManagerModel: await read("apps/macos/Sources/SkillsCopilot/Models/SkillManager.swift"),
  batchSkillOperation: await read("apps/macos/Sources/SkillsCopilot/Views/BatchSkillOperationSheet.swift"),
  markdownRender: await read("apps/macos/Sources/SkillsCopilot/Models/MarkdownRenderDocument.swift"),
  markdownTableDisplay: await read("apps/macos/Sources/SkillsCopilot/Models/MarkdownTableDisplayModel.swift"),
  agentConfigWorkspace: await read("apps/macos/Sources/SkillsCopilot/Views/AgentConfigWorkspacePanel.swift"),
  detailHeaderOverview: await read("apps/macos/Sources/SkillsCopilot/Views/DetailHeaderOverviewSection.swift"),
  detailFindingsHistory: await read("apps/macos/Sources/SkillsCopilot/Views/DetailFindingsHistorySection.swift"),
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
  revisionAutosave: await read("apps/macos/Sources/SkillsCopilot/Models/RevisionAutosaveCoordinator.swift"),
  localSessionCache: await read("apps/macos/Sources/SkillsCopilot/Models/LocalSessionCache.swift"),
  listCompletenessControls: await read("apps/macos/Sources/SkillsCopilot/Views/ListCompletenessControls.swift"),
  store: await read("apps/macos/Sources/SkillsCopilot/Stores/SkillStore.swift"),
  storeList: await read("apps/macos/Sources/SkillsCopilot/Stores/SkillListModel.swift"),
  storeDerivedState: await read("apps/macos/Sources/SkillsCopilot/Stores/SkillStoreDerivedState.swift"),
  storeWorkflow: await read("apps/macos/Sources/SkillsCopilot/Stores/SkillStoreWorkflowSelectors.swift"),
  taskCockpit: await read("apps/macos/Sources/SkillsCopilot/Views/TaskCockpitPanel.swift"),
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
files.detailSurface = [
  files.detail,
  files.agentSessionDetail,
  files.detailSection,
  files.detailOverview,
  files.detailPrimitives,
  files.providerObservabilitySettings,
  files.agentConfigWorkspace,
  files.detailHeaderOverview,
  files.detailFindingsHistory,
  files.taskCockpit,
].join("\n");
files.serviceIPC = [
  files.serviceClient,
  files.serviceClientTransport,
  files.serviceProcessRunner,
].join("\n");
files.storeSurface = [
  files.store,
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
    label: "detail event history exposes retryable completeness actions",
    text: files.detail + "\n" + files.detailHeaderOverview,
    passed: /HistorySection\([\s\S]*?completeness:\s*store\.selectedSkillEventCompleteness[\s\S]*?onLoadMore:[\s\S]*?loadMoreSkillEvents[\s\S]*?onLoadAll:[\s\S]*?loadMoreSkillEvents[\s\S]*?onCancel:[\s\S]*?cancelSkillEventLoadAll/.test(files.detail)
      && /struct HistorySection:[\s\S]*?let completeness:\s*ListCompletenessState[\s\S]*?ListCompletenessFooter\([\s\S]*?state:\s*completeness/.test(files.detailHeaderOverview),
  },
  {
    label: "findings and conflicts expose catalog scan completeness",
    text: files.detail + "\n" + files.detailFindingsHistory,
    passed: /FindingsSection\([\s\S]*?catalogCompleteness:\s*store\.catalogListCompleteness/.test(files.detail)
      && /struct FindingsSection:[\s\S]*?let catalogCompleteness:\s*ListCompletenessState[\s\S]*?ListCompletenessFooter/.test(files.detailFindingsHistory)
      && /if findings\.isEmpty && conflicts\.isEmpty\s*\{\s*if catalogCompleteness\.completeness == \.complete\s*\{[\s\S]*?UIStrings\.noFindings/.test(files.detailFindingsHistory)
      && /catalogCompletenessState\([\s\S]*?loadedCount:\s*findings\.count/.test(files.detailFindingsHistory)
      && /catalogCompletenessState\([\s\S]*?loadedCount:\s*conflicts\.count/.test(files.detailFindingsHistory),
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
    label: "application termination waits for pending and active autosaves",
    text: files.app + "\n" + files.store,
    passed: /applicationShouldTerminate\(_ sender:\s*NSApplication\)[\s\S]*?\.terminateLater[\s\S]*?flushPendingAutosaves\(\)[\s\S]*?reply\(toApplicationShouldTerminate:\s*true\)/.test(files.app)
      && /hasRepliedToTerminationRequest = false[\s\S]*?guard !hasRepliedToTerminationRequest[\s\S]*?self\.hasRepliedToTerminationRequest = true[\s\S]*?reply\(toApplicationShouldTerminate:\s*true\)/.test(files.app)
      && /configureAutosaveFlusher\([\s\S]*?store:\s*SkillStore/.test(files.app)
      && /appDelegate\.configureAutosaveFlusher\(store:\s*store\)/.test(files.app),
  },
  {
    label: "autosave mutation lane durably cancels pre-owner revision tokens and drains them on shutdown",
    text: files.revisionAutosave + "\n" + files.store,
    passed: /struct AutosaveMutationLaneToken:[\s\S]*?case config[\s\S]*?case provider[\s\S]*?let revision:\s*UInt64/.test(files.revisionAutosave)
      && /enum AutosaveMutationLaneResult<Result>[\s\S]*?case completed\(Result\)[\s\S]*?case cancelled/.test(files.revisionAutosave)
      && /currentOwnerToken:\s*AutosaveMutationLaneToken\?/.test(files.revisionAutosave)
      && /registeredTokens:\s*Set<AutosaveMutationLaneToken>/.test(files.revisionAutosave)
      && /cancelledTokens:\s*Set<AutosaveMutationLaneToken>/.test(files.revisionAutosave)
      && /func register\(_ token:\s*AutosaveMutationLaneToken\)[\s\S]*?registeredTokens\.insert\(token\)/.test(files.revisionAutosave)
      && /func cancelQueued\(_ token:\s*AutosaveMutationLaneToken\)[\s\S]*?currentOwnerToken != token[\s\S]*?cancelledTokens\.insert\(token\)[\s\S]*?waiters\.remove\(at:\s*index\)[\s\S]*?resume\(returning:\s*false\)/.test(files.revisionAutosave)
      && /private func acquire\(token:[\s\S]*?cancelledTokens\.remove\(token\)[\s\S]*?currentOwnerToken = token/.test(files.revisionAutosave)
      && /private func enqueue\(token:[\s\S]*?cancelledTokens\.remove\(token\)[\s\S]*?waiters\.append/.test(files.revisionAutosave)
      && /func shutdown\(\)[\s\S]*?waiters\.removeAll\(\)[\s\S]*?resume\(returning:\s*false\)/.test(files.revisionAutosave)
      && /waiters\.removeFirst\(\)\.continuation\.resume\(returning:\s*true\)/.test(files.revisionAutosave)
      && /workerWillStart:\s*\{[\s\S]*?autosaveMutationLane\.register\([\s\S]*?family:\s*\.config/.test(files.store)
      && /workerWillStart:\s*\{[\s\S]*?autosaveMutationLane\.register\([\s\S]*?family:\s*\.provider/.test(files.store)
      && /submitConfigAutosave\(content:[\s\S]*?validationError != nil[\s\S]*?cancelQueued/.test(files.store)
      && /submitProviderAutosave\(draft:[\s\S]*?draft\.validationMessage != nil[\s\S]*?cancelQueued/.test(files.store)
      && /cancelPendingConfigAutosave\(\)[\s\S]*?AutosaveMutationLaneToken\(family:\s*\.config,\s*revision:\s*activeRevision\)/.test(files.store)
      && /cancelPendingProviderAutosave\(\)[\s\S]*?AutosaveMutationLaneToken\(family:\s*\.provider,\s*revision:\s*activeRevision\)/.test(files.store)
      && /deinit[\s\S]*?lane\.shutdown\(\)/.test(files.store),
  },
  {
    label: "formal summaries and global search expose stable full-access controls",
    text: [
      files.sidebar,
      files.batchSkillOperation,
      files.detailOverview,
      files.taskCockpit,
      files.skillManager,
      files.detailPrimitives,
      files.content,
    ].join("\n"),
    passed: [
      "session-top-skills.show-all",
      "batch-toggle-items.show-all",
      "permission-summary.show-all",
      "task-cockpit-candidates.show-all",
      "task-cockpit-context.show-all",
      "skill-manager-agents.show-all",
      "markdown-table.show-all",
      "global-search.skills.view-all",
      "global-search.sessions.view-all",
      "global-search.config-history.view-all",
    ].every((identifier) => [
      files.sidebar,
      files.batchSkillOperation,
      files.detailOverview,
      files.taskCockpit,
      files.skillManager,
      files.detailPrimitives,
      files.content,
    ].some((source) => source.includes(identifier)))
      && /private struct TaskCockpitTechnicalDiagnosticsView:[\s\S]*?TaskCockpitCandidateList\([\s\S]*?routeCandidates[\s\S]*?TaskCockpitCandidateList\([\s\S]*?agentCandidates[\s\S]*?TaskCockpitCandidateList\([\s\S]*?skillCandidates[\s\S]*?TaskCockpitContextList\([\s\S]*?gapRows[\s\S]*?TaskCockpitContextList\([\s\S]*?blockerRows[\s\S]*?TaskCockpitEvidenceList\([\s\S]*?evidenceReferences[\s\S]*?TaskCockpitSafetyList\(/.test(files.taskCockpit)
      && /private struct SkillManagerTargetSummary:[\s\S]*?ExpandableSummaryList\([\s\S]*?columns:\s*\[GridItem\(\.adaptive/.test(files.skillManager),
  },
  {
    label: "main shell uses NavigationSplitView",
    text: files.content,
    pattern: /NavigationSplitView(?:\([\s\S]*?\))?\s*{/,
  },
  {
    label: "list pages use a unified window toolbar with global search and sidebar-local selectors",
    text: files.content + "\n" + files.uiOptimization,
    passed: /static let unifiedToolbar = UnifiedToolbarPresentation\(\)[\s\S]*?static let listPage = ListPagePresentation\(\)[\s\S]*?static let sidebarShell = SidebarShellPresentation\(\)/.test(files.uiOptimization)
      && /struct UnifiedToolbarPresentation:[\s\S]*?spansEntireWindow = true[\s\S]*?searchPlacement = UnifiedToolbarSearchPlacement\.globalTrailing[\s\S]*?collapsesAtScrollEdge = true[\s\S]*?settingsActionUsesSystemSettingsLink = true/.test(files.uiOptimization)
      && /struct ListPagePresentation:[\s\S]*?filterStyle = ListPageFilterStyle\.capsule[\s\S]*?searchScope = ListPageSearchScope\.localList[\s\S]*?rowStyle = ListPageRowStyle\.whiteCard[\s\S]*?minimumCardRowHeight = 58[\s\S]*?cardRowSpacing = 8/.test(files.uiOptimization)
      && /ZStack\(alignment:\s*\.topTrailing\)[\s\S]*?if shouldShowGlobalSearchResultsOverlay[\s\S]*?globalSearchResultsOverlay[\s\S]*?pinnedWindowChromeControls/.test(files.content)
      && /private var appShell:\s*some View\s*\{[\s\S]*?navigationShell[\s\S]*?\n\s*\}/.test(files.content)
      && /private var pinnedWindowChromeControls:\s*some View\s*\{[\s\S]*?WindowChromeTitlebarAccessory\s*\{[\s\S]*?WindowChromeToolbarControls\([\s\S]*?text:\s*\$globalSearchText,[\s\S]*?isSearchFocused:\s*\$isGlobalSearchFocused,[\s\S]*?showsSearchResults:\s*\$showsGlobalSearchResults,[\s\S]*?onSubmit:\s*selectFirstGlobalSearchResult[\s\S]*?\.frame\(width:\s*0,\s*height:\s*0\)[\s\S]*?\.allowsHitTesting\(false\)[\s\S]*?\.accessibilityHidden\(true\)[\s\S]*?\.zIndex\(10\)/.test(files.content)
      && /@State private var isGlobalSearchFocused = false[\s\S]*?@State private var showsGlobalSearchResults = false/.test(files.content)
      && /private var globalSearchResultsOverlay:[\s\S]*?GlobalSearchResultsOverlay\([\s\S]*?query:\s*trimmedGlobalSearchText,[\s\S]*?results:\s*globalSearchResults,[\s\S]*?kindCounts:\s*store\.appSearchResult\.kindCounts[\s\S]*?onViewAll:\s*showAllGlobalSearchResults[\s\S]*?selectGlobalSearchResult\(result\)[\s\S]*?WindowChromeToolbarMetrics\.searchResultsTrailingPadding/.test(files.content)
      && /SecondarySidebarView\(columnVisibility:\s*columnVisibility\)/.test(files.content)
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
      && /private struct WindowChromeToolbarControls:\s*View[\s\S]*?HStack\(spacing:\s*8\)\s*\{\s*TitlebarAgentSelectorControl\(\)[\s\S]*?\.frame\(width:\s*agentWidth,\s*height:\s*controlHeight,\s*alignment:\s*\.leading\)\s*TitlebarProjectPickerControl\(isCompact:\s*false\)[\s\S]*?\.frame\(width:\s*projectWidth,\s*height:\s*controlHeight,\s*alignment:\s*\.leading\)\s*WindowChromeTrailingControls\([\s\S]*?text:\s*\$text[\s\S]*?private var controlHeight:\s*CGFloat \{ WindowChromeToolbarMetrics\.controlHeight \}[\s\S]*?private var agentWidth:\s*CGFloat \{ WindowChromeToolbarMetrics\.agentWidth \}[\s\S]*?private var projectWidth:\s*CGFloat \{ WindowChromeToolbarMetrics\.projectWidth \}/.test(files.content)
      && !extractStructBody(files.content, "WindowChromeToolbarControls").includes("Divider()")
      && !/private struct WindowChromeToolbarControls:[\s\S]*?columnVisibility|isPrimarySidebarCollapsed/.test(files.content)
      && !/GlassEffectContainer|glassEffect\(/.test(files.content)
      && !/\.toolbar\s*\{[\s\S]*?ToolbarItem\(placement:\s*\.navigation\)[\s\S]*?TitlebarAgentSelectorControl\(\)/.test(files.content)
      && /private struct TitlebarAgentSelectorControl:\s*View[\s\S]*?isPopoverPresented\.toggle\(\)[\s\S]*?TitlebarAgentSelectorLabel\([\s\S]*?\.popover\(isPresented:\s*\$isPopoverPresented[\s\S]*?ForEach\(SkillAgentFilter\.managementCases\)[\s\S]*?store\.agentFilter = filter/.test(files.content)
      && /private struct TitlebarProjectPickerControl:\s*View[\s\S]*?Button\s*\{[\s\S]*?isPopoverPresented\.toggle\(\)[\s\S]*?\.popover\(isPresented:\s*\$isPopoverPresented[\s\S]*?Button\s*\{[\s\S]*?chooseProject\(\)[\s\S]*?ForEach\(store\.recentProjectContexts\)[\s\S]*?await store\.setProject\([\s\S]*?revealActiveProject\(\)[\s\S]*?await store\.clearProject\(\)/.test(files.content)
      && /struct SecondarySidebarView:[\s\S]*?let columnVisibility:\s*NavigationSplitViewVisibility[\s\S]*?List\(selection:\s*\$store\.selectedSidebarSelection\)[\s\S]*?\.padding\(\.top,\s*50\)[\s\S]*?\.ignoresSafeArea\(\.container,\s*edges:\s*\.top\)[\s\S]*?GeometryReader \{ proxy in[\s\S]*?SecondarySidebarHeaderWidthPreferenceKey\.self[\s\S]*?\.allowsHitTesting\(false\)[\s\S]*?\.navigationTitle\(""\)/.test(files.sidebar)
      && !/\.overlay\(alignment:\s*\.topLeading\)[\s\S]*?SecondarySidebarHeaderChrome/.test(files.sidebar)
      && !/ToolbarItemGroup\(placement:\s*\.automatic\)[\s\S]*?Global/.test(files.content)
      && /private struct WindowChromeTrailingControls:[\s\S]*?private let searchWidth = WindowChromeToolbarMetrics\.searchWidth[\s\S]*?private var controls:[\s\S]*?HStack\(alignment:\s*\.center,\s*spacing:\s*6\)[\s\S]*?GlobalWindowSearchControl\([\s\S]*?WindowChromeAboutButton\(\)[\s\S]*?WindowChromeSettingsControl\(\)[\s\S]*?\.frame\(height:\s*32,\s*alignment:\s*\.center\)/.test(files.content)
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
      && /private struct WindowChromeAboutButton:[\s\S]*?NSApp\.orderFrontStandardAboutPanel\(nil\)[\s\S]*?questionmark\.circle[\s\S]*?frame\(width:\s*30,\s*height:\s*30\)[\s\S]*?\.windowChromeGlassCircle\(\)/.test(files.content)
      && /private struct WindowChromeSettingsControl:[\s\S]*?if #available\(macOS 14\.0,\s*\*\)[\s\S]*?SettingsLink[\s\S]*?settingsLabel[\s\S]*?Button\(action:\s*openSettingsFallback\)/.test(files.content)
      && /private struct WindowChromeSettingsControl:[\s\S]*?\.windowChromeGlassCircle\(\)[\s\S]*?gearshape[\s\S]*?frame\(width:\s*30,\s*height:\s*30\)[\s\S]*?openSettingsFallback\(\)[\s\S]*?showPreferencesWindow/.test(files.content)
      && /private extension View[\s\S]*?func windowChromeGlassCapsule\(\)[\s\S]*?Color\(nsColor:\s*\.controlBackgroundColor\)\.opacity\(0\.72\)[\s\S]*?func windowChromeGlassCircle\(\)[\s\S]*?Color\(nsColor:\s*\.controlBackgroundColor\)\.opacity\(0\.72\)/.test(files.content)
      && /struct SecondarySidebarHeaderWidthPreferenceKey:\s*PreferenceKey[\s\S]*?struct SecondarySidebarHeaderChrome:\s*View[\s\S]*?let availableWidth:\s*CGFloat[\s\S]*?let agentLeading = agentLeadingInset\(for:\s*availableWidth\)[\s\S]*?let projectLeading = projectLeadingInset\(for:\s*availableWidth,\s*agentLeading:\s*agentLeading\)[\s\S]*?let agentFrame = CGRect\([\s\S]*?let projectFrame = CGRect\([\s\S]*?ZStack\(alignment:\s*\.topLeading\)[\s\S]*?SecondarySidebarAgentHeaderControl\(\)[\s\S]*?\.offset\(x:\s*agentLeading,[\s\S]*?SecondarySidebarProjectHeaderControl\(isCompact:\s*isPrimarySidebarCollapsed\)[\s\S]*?\.offset\(x:\s*projectLeading,[\s\S]*?\.contentShape\([\s\S]*?SecondarySidebarHeaderHitShape\([\s\S]*?agentFrame:\s*agentFrame,[\s\S]*?projectFrame:\s*projectFrame[\s\S]*?private func agentLeadingInset\(for availableWidth:[\s\S]*?private func projectLeadingInset\(for availableWidth:[\s\S]*?private struct SecondarySidebarHeaderHitShape:\s*Shape[\s\S]*?path\.addRoundedRect\(in:\s*agentFrame[\s\S]*?path\.addRoundedRect\(in:\s*projectFrame/.test(files.sidebar)
      && /struct SecondarySidebarAgentHeaderControl:\s*View[\s\S]*?SecondarySidebarAgentSelectorMenu\(\)[\s\S]*?frame\(minWidth:\s*126,\s*idealWidth:\s*148,\s*maxWidth:\s*158/.test(files.sidebar)
      && /struct SecondarySidebarProjectHeaderControl:\s*View[\s\S]*?let isCompact:\s*Bool[\s\S]*?SecondarySidebarProjectPickerMenu\(isCompact:\s*isCompact\)[\s\S]*?minWidth:\s*isCompact \? 36 : 42[\s\S]*?idealWidth:\s*isCompact \? 36 : 140[\s\S]*?maxWidth:\s*isCompact \? 36 : 152/.test(files.sidebar)
      && !/SecondarySidebarProjectPickerMenu\(isCompact:\s*true\)[\s\S]*?frame\(maxWidth:\s*\.infinity,\s*alignment:\s*\.trailing\)/.test(files.sidebar)
      && /private struct SecondarySidebarAgentSelectorMenu:[\s\S]*?Menu\s*\{[\s\S]*?ForEach\(SkillAgentFilter\.managementCases\)[\s\S]*?store\.agentFilter = filter[\s\S]*?SecondarySidebarAgentSelectorLabel\([\s\S]*?shortTitle\(for:\s*store\.agentFilter\)[\s\S]*?\.accessibilityValue\(store\.agentFilter\.title\)/.test(files.sidebar)
      && /private struct SecondarySidebarAgentSelectorLabel:[\s\S]*?AgentIconBadge\(filter:\s*filter,\s*size:\s*24\)[\s\S]*?Image\(systemName:\s*"chevron\.up\.chevron\.down"\)[\s\S]*?\.frame\(minWidth:\s*126[\s\S]*?\.secondarySidebarHeaderControlCapsule\(\)/.test(files.sidebar)
      && /private struct SecondarySidebarProjectPickerMenu:[\s\S]*?Menu\s*\{[\s\S]*?Label\(UIStrings\.chooseProject,\s*systemImage:\s*"folder\.badge\.plus"\)[\s\S]*?Section\(UIStrings\.recentProjects\)[\s\S]*?await store\.setProject\([\s\S]*?Label\(UIStrings\.revealInFinder,[\s\S]*?arrow\.up\.forward\.app[\s\S]*?Label\(UIStrings\.clearProject,[\s\S]*?xmark\.circle[\s\S]*?SecondarySidebarProjectPickerLabel\([\s\S]*?title:\s*projectTitle[\s\S]*?return UIStrings\.toolbarNoProjectSelected[\s\S]*?private var projectHelp:[\s\S]*?DisplayText\.privacyPath\(rootPath,\s*privacyModeEnabled:\s*true\)/.test(files.sidebar)
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
      && /struct AppStartupLoadingState:[\s\S]*?let message: String[\s\S]*?let progress: Double/.test(files.store)
      && /@Published private\(set\) var startupLoadingState:[\s\S]*?UIStrings\.startupPreparingLoading/.test(files.store)
      && /@Published private\(set\) var hasCompletedStartupLoad = false/.test(files.store)
      && /func loadAppStartupDataIfNeeded\(\) async[\s\S]*?try await refreshCollections\(includeSupplementalData:\s*false,\s*includeAIProviderStatus:\s*false\)[\s\S]*?await loadSelectedDetail\(\)[\s\S]*?scheduleStartupSupplementalLoads\(/.test(files.store)
      && /private func scheduleStartupSupplementalLoads\([\s\S]*?loadLocalSessions:\s*true[\s\S]*?loadAgentConfigDocuments:\s*true[\s\S]*?forceProviderObservability:\s*false/.test(files.store)
      && /private func schedulePostRefreshSupplementalLoads\([\s\S]*?await self\.loadAIProviderStatusIfNeeded\(\)[\s\S]*?await self\.refreshSelectedAgentLocalSessionsIfNeeded\(\)[\s\S]*?await self\.loadCurrentAgentConfigDocumentsIfNeeded\(agent:\s*requestedAgentFilter\.rawValue\)[\s\S]*?await self\.loadLLMPromptRuns\(\)[\s\S]*?await self\.loadProviderObservabilityDuringRefresh\(force:\s*forceProviderObservability\)/.test(files.store)
      && /"startup\.catalog" = "Loading catalog data\.\.\."/.test(files.localizable),
  },
  {
    label: "primary and secondary sidebar columns have bounded native widths",
    text: files.content + "\n" + files.uiOptimization,
    passed: /struct SidebarShellPresentation:[\s\S]*?let width = 260/.test(files.uiOptimization)
      && /minimumSecondaryColumnWidth = 360[\s\S]*?idealSecondaryColumnWidth = 400[\s\S]*?maximumSecondaryColumnWidth = 520/.test(files.uiOptimization)
      && /SidebarView\(\)[\s\S]*?UIOptimizationPresentation\.sidebarShell\.width[\s\S]*?UIOptimizationPresentation\.sidebarShell\.width[\s\S]*?UIOptimizationPresentation\.sidebarShell\.width[\s\S]*?SecondarySidebarView\(columnVisibility:\s*columnVisibility\)[\s\S]*?UIOptimizationPresentation\.skillList\.minimumSecondaryColumnWidth[\s\S]*?UIOptimizationPresentation\.skillList\.idealSecondaryColumnWidth[\s\S]*?UIOptimizationPresentation\.skillList\.maximumSecondaryColumnWidth/.test(files.content),
  },
  {
    label: "selected agent session metrics refresh from the root view uses need-based prewarm",
    text: files.content + "\n" + files.storeSurface,
    pattern: /(?=[\s\S]*?\.task\(id:\s*store\.selectedAgentLocalSessionRefreshKey\)[\s\S]*?await store\.refreshSelectedAgentLocalSessionsIfNeeded\(\))(?=[\s\S]*?var selectedAgentLocalSessionRefreshKey:[\s\S]*?agentFilter\.rawValue[\s\S]*?activeProjectContext\?\.rootPath)(?=[\s\S]*?func refreshSelectedAgentLocalSessionsIfNeeded\(\)\s*async[\s\S]*?refreshLocalSessionSnapshot\(reason:\s*\.sourceChanged\))/,
  },
  {
    label: "primary sidebar exposes agent cards plus global skill manager and preflight footer tools",
    text: files.sidebar + "\n" + files.detail + "\n" + files.app + "\n" + files.mainWindowCoordinator + "\n" + files.workflowSheet,
    passed: /@State private var isSkillManagerSheetPresented = false/.test(files.sidebar)
      && /@State private var isPreflightSheetPresented = false/.test(files.sidebar)
      && /List\s*\{[\s\S]*?Section\(UIStrings\.text\("sidebar\.primaryNavigation"/.test(files.sidebar)
      && !/ProjectContextControls\(\)/.test(files.sidebar)
      && /SidebarNavigationCardButton\([\s\S]*?title:\s*SidebarContentMode\.sessions\.title[\s\S]*?sessionCardMetrics[\s\S]*?selectSessions\(\)/.test(files.sidebar)
      && /SidebarNavigationCardButton\([\s\S]*?title:\s*SidebarContentMode\.skills\.title[\s\S]*?skillCardMetrics[\s\S]*?selectSkills\(\)/.test(files.sidebar)
      && /SidebarNavigationCardButton\([\s\S]*?title:\s*SidebarContentMode\.config\.title[\s\S]*?configCardMetrics[\s\S]*?selectConfig\(\)/.test(files.sidebar)
      && /SidebarFooterToolRow\([\s\S]*?isSkillManagerPresented:\s*isSkillManagerSheetPresented[\s\S]*?onOpenSkillManager:[\s\S]*?isSkillManagerSheetPresented = true[\s\S]*?onOpenPreflight:[\s\S]*?isPreflightSheetPresented = true[\s\S]*?\)/.test(files.sidebar)
      && /private struct SidebarFooterToolRow:[\s\S]*?skillManager\.title[\s\S]*?skillManager\.sidebar\.subtitle[\s\S]*?sidebar\.skillManager\.metric\.global[\s\S]*?UIStrings\.taskCockpitTitle[\s\S]*?sidebar\.preflight\.subtitle/.test(files.sidebar)
      && /\.sheet\(isPresented:\s*\$isSkillManagerSheetPresented\)[\s\S]*?SkillPackageManagerSheet\(\)/.test(files.sidebar + "\n" + files.content)
      && /struct SkillPackageManagerSheet:[\s\S]*?WorkflowSheetShell\([\s\S]*?SkillManagerPanel\(showsHeader:\s*false\)/.test(files.sidebar)
      && /\.sheet\(isPresented:\s*\$isPreflightSheetPresented\)[\s\S]*?TaskPreflightPreviewSheet\(\)/.test(files.sidebar)
      && /\.navigationTitle\(""\)/.test(files.detail)
      && /window\.titleVisibility = \.hidden/.test(files.mainWindowCoordinator)
      && /window\.titlebarAppearsTransparent = true/.test(files.mainWindowCoordinator)
      && /window\.styleMask\.insert\(\.fullSizeContentView\)/.test(files.mainWindowCoordinator)
      && /\.padding\(\.top,\s*8\)[\s\S]*?\.padding\(\.horizontal,\s*28\)[\s\S]*?\.padding\(\.bottom,\s*28\)/.test(files.detail)
      && !/Section\(UIStrings\.text\("skillManager\.title"[\s\S]*?SidebarSelection\.work\(\.skillManager\)/.test(files.sidebar)
      && !/selectedSidebarSelection\s*=\s*\.work\(\.skillManager\)/.test(files.sidebar)
      && !/selectedDetailSection == \.skillManager[\s\S]*?SkillManagerPanel\(\)/.test(files.detail)
      && !/navigationTitle\(UIStrings\.appWindowTitle\)/.test(files.detail)
      && !/SidebarNavigationCardButton\([\s\S]*?UIStrings\.taskCockpitTitle[\s\S]*?selectedSidebarSelection = \.preflight/.test(files.sidebar)
      && !/selectedSidebarSelection\s*=\s*\.preflight/.test(files.sidebar),
  },
  {
    label: "secondary sidebar omits the agent profile row and switches session, skill, or config lists",
    text: files.sidebar,
    passed: /struct SecondarySidebarView:[\s\S]*?List\(selection:\s*\$store\.selectedSidebarSelection\)[\s\S]*?switch store\.sidebarContentMode[\s\S]*?case \.sessions:[\s\S]*?SessionSidebarPanel\(\)[\s\S]*?case \.skills:[\s\S]*?SkillSidebarPanel/.test(files.sidebar)
      && /case \.config:[\s\S]*?ConfigSidebarPanel\(\)/.test(files.sidebar)
      && !/AgentProfileSidebarRow/.test(files.sidebar)
      && !/SidebarSelection\.agentWorkspace/.test(files.sidebar),
  },
  {
    label: "sidebar sessions surface exposes refresh, compact rows, and top skill usage",
    text: files.sidebar + "\n" + files.store,
    passed: /private struct SessionSidebarPanel:[\s\S]*?let preview = store\.localSessionPreviewResult[\s\S]*?sidebar\.sessions\.list[\s\S]*?SessionSidebarRow\([\s\S]*?showsProjectRoot:\s*store\.localSessionScopeFilter == \.all[\s\S]*?store\.selectedSidebarSelection == \.session\(session\.id\)[\s\S]*?store\.selectLocalSession\(session\)[\s\S]*?preview\.skillUsageRows/.test(files.sidebar)
      && /private var sessionRefreshButton:[\s\S]*?await store\.previewLocalSessions\(\)/.test(files.sidebar)
      && /private struct SessionSidebarRow:[\s\S]*?let showsProjectRoot:\s*Bool[\s\S]*?session\.projectRoot[\s\S]*?if let startedAt = session\.startedAt[\s\S]*?sidebar\.sessions\.startShort[\s\S]*?if let endedAt = session\.endedAt[\s\S]*?sidebar\.sessions\.lastShort/.test(files.sidebar)
      && /private func selectSessions\(\)[\s\S]*?refreshSelectedAgentLocalSessionsIfNeeded\(\)/.test(files.sidebar)
      && /private func selectSessions\(\)[\s\S]*?store\.filteredLocalSessionRows\.first/.test(files.sidebar)
      && !/private func selectSessions\(\)[\s\S]*?localSessionPreviewResult\.sessionRows\.first/.test(files.sidebar)
      && /private var sessionStatusMessage:[\s\S]*?fallbackReason[\s\S]*?authorizationRequired[\s\S]*?return nil/.test(files.sidebar)
      && !/private var sessionStatusMessage:[\s\S]*?UIStrings\.loading[\s\S]*?return nil/.test(files.sidebar)
      && /@Published var localSessionScopeFilter:[\s\S]*?guard oldValue != localSessionScopeFilter else \{ return \}[\s\S]*?normalizeSelectedLocalSession\(\)/.test(files.store)
      && /func refreshLocalSessionSnapshot\(reason:\s*LocalSessionRefreshReason\) async[\s\S]*?service\.previewLocalSessions\([\s\S]*?scope:\s*\.all[\s\S]*?search:\s*nil[\s\S]*?sessionID:\s*nil[\s\S]*?includeContentItems:\s*false[\s\S]*?limit:\s*Self\.localSessionPageLimit[\s\S]*?offset:\s*nil[\s\S]*?cursor:\s*cursor[\s\S]*?sourceRevision:\s*sourceRevision[\s\S]*?sort:\s*\.recent[\s\S]*?direction:\s*\.descending/.test(files.store)
      && /func loadMoreLocalSessions\(\) async[\s\S]*?continueLocalSessionPages\(loadAll:\s*false\)/.test(files.store)
      && /func loadAllLocalSessions\(\) async[\s\S]*?continueLocalSessionPages\(loadAll:\s*true\)/.test(files.store)
      && /ListCompletenessFooter\([\s\S]*?loadMoreLocalSessions\(\)[\s\S]*?loadAllLocalSessions\(\)[\s\S]*?cancelLocalSessionLoadAll\(\)/.test(files.sidebar)
      && /private func localSessionSnapshotKey\(roots:[\s\S]*?LocalSessionSnapshotKey\([\s\S]*?projectRoot:\s*activeProjectContext\?\.rootPath[\s\S]*?authorizedRoots:\s*roots/.test(files.store)
      && !/private func localSessionSnapshotKey\(roots:[\s\S]*?localSessionScopeFilter\.rawValue/.test(files.store)
      && /func refreshSelectedAgentLocalSessionsIfNeeded\(\) async[\s\S]*?refreshLocalSessionSnapshot\(reason:\s*\.sourceChanged\)/.test(files.store)
      && /\.task\(id:\s*store\.selectedAgentLocalSessionRefreshKey\)[\s\S]*?refreshSelectedAgentLocalSessionsIfNeeded\(\)/.test(files.content)
      && /func selectLocalSession\([\s\S]*?_ session:\s*LocalSessionPreviewRow,[\s\S]*?origin:\s*LocalSessionSelectionOrigin = \.user[\s\S]*?setSidebarSelection\(\.session\(session\.id\)\)[\s\S]*?loadLocalSessionDetailIfNeeded\(sessionID:\s*sessionID\)/.test(files.store)
      && /func loadLocalSessionDetailIfNeeded\(sessionID:\s*String\) async[\s\S]*?sessionID:\s*sessionID[\s\S]*?includeContentItems:\s*true[\s\S]*?limit:\s*1[\s\S]*?localSessionCache\.publishDetail/.test(files.store)
      && !/sessionTimeRangeSummary/.test(files.sidebar),
  },
  {
    label: "session empty states explain filtered counts without secondary-sidebar wording",
    text: files.sidebar + "\n" + files.uiStrings + "\n" + files.localizable,
    passed: /else if filteredRows\.isEmpty[\s\S]*?SidebarEmptyMessage\(message:\s*UIStrings\.localSessionNoMatchesMessage\(totalCount:\s*preview\.totalMatchedCount\)\)/.test(files.sidebar)
      && /static func localSessionNoMatchesMessage\(totalCount:\s*Int\)[\s\S]*?sidebar\.sessions\.noMatchesWithCount/.test(files.uiStrings)
      && /"sidebar\.sessions\.noMatchesWithCount"\s*=/.test(files.localizable)
      && !/empty\.noSessionSelected\.message" = ".*secondary sidebar/.test(files.localizable),
  },
  {
    label: "skill sidebar exposes filter scope sort and direction controls",
    text: files.sidebar,
    pattern: /private struct SkillSidebarPanel:[\s\S]*?skillToolbar\(visibleSkills:\s*visibleSkills\)[\s\S]*?private var filterControls:[\s\S]*?selection:\s*\$store\.stateFilter[\s\S]*?SkillStateFilter\.sidebarCases[\s\S]*?selection:\s*\$store\.skillScopeFilter[\s\S]*?selection:\s*\$store\.sortOrder[\s\S]*?store\.sortDirection = store\.sortDirection == \.ascending \? \.descending : \.ascending/,
  },
  {
    label: "skill sidebar filter controls use compact adaptive sizing",
    text: files.sidebar + "\n" + files.uiOptimization,
    passed: /struct SkillListPresentation:[\s\S]*?let filterControlWidth = 72[\s\S]*?let filterControlHeight = 28[\s\S]*?let filterControlSpacing = 4[\s\S]*?let filterToolbarVerticalPadding = 4[\s\S]*?let sortDirectionButtonWidth = 28/.test(files.uiOptimization)
      && /private var filterControls:[\s\S]*?let layout = UIOptimizationPresentation\.skillList[\s\S]*?HStack\(alignment:\s*\.center,\s*spacing:\s*CGFloat\(layout\.filterControlSpacing\)\)[\s\S]*?SkillFilterMenuPicker\([\s\S]*?selection:\s*\$store\.stateFilter[\s\S]*?options:\s*SkillStateFilter\.sidebarCases[\s\S]*?width:\s*CGFloat\(layout\.filterControlWidth\)[\s\S]*?height:\s*CGFloat\(layout\.filterControlHeight\)[\s\S]*?SkillFilterMenuPicker\([\s\S]*?selection:\s*\$store\.skillScopeFilter[\s\S]*?options:\s*SkillScopeFilter\.allCases[\s\S]*?width:\s*CGFloat\(layout\.filterControlWidth\)[\s\S]*?height:\s*CGFloat\(layout\.filterControlHeight\)[\s\S]*?SkillFilterMenuPicker\([\s\S]*?selection:\s*\$store\.sortOrder[\s\S]*?options:\s*SkillSortOrder\.allCases[\s\S]*?width:\s*CGFloat\(layout\.filterControlWidth\)[\s\S]*?height:\s*CGFloat\(layout\.filterControlHeight\)[\s\S]*?sortDirectionButton\(width:\s*CGFloat\(layout\.sortDirectionButtonWidth\),\s*height:\s*CGFloat\(layout\.filterControlHeight\)\)[\s\S]*?\.padding\(\.vertical,\s*CGFloat\(layout\.filterToolbarVerticalPadding\)\)/.test(files.sidebar)
      && /private struct SkillFilterMenuPicker<Option:[\s\S]*?var expands = true[\s\S]*?SidebarMenuButtonLabel\([\s\S]*?title:\s*title,[\s\S]*?value:\s*optionTitle\(selection\),[\s\S]*?expands:\s*expands/.test(files.sidebar)
      && /private struct SidebarMenuButtonLabel:[\s\S]*?\.frame\(minWidth:\s*width,\s*maxWidth:\s*expands \? \.infinity : nil,\s*minHeight:\s*height,\s*maxHeight:\s*height[\s\S]*?\.fixedSize\(horizontal:\s*!expands,\s*vertical:\s*false\)[\s\S]*?in:\s*Capsule\(\)/.test(files.sidebar)
      && /private struct SkillFilterMenuPicker<Option:[\s\S]*?Menu\s*\{[\s\S]*?ForEach\(options\)[\s\S]*?Button \{[\s\S]*?selection = option[\s\S]*?\.menuStyle\(\.button\)[\s\S]*?\.buttonStyle\(\.plain\)/.test(files.sidebar)
      && !/private struct SkillFilterMenuPicker<Option:[\s\S]*?\.popover\(isPresented:/.test(files.sidebar),
  },
  {
    label: "skill rows expose issue badges and navigation affordance",
    text: files.sidebar + "\n" + files.storeList,
    passed: /SkillRow\([\s\S]*?skill:\s*skill,[\s\S]*?issueCount:\s*store\.issueIndicatorCount\(for:\s*skill\),[\s\S]*?isSelected:\s*store\.selectedSidebarSelection == \.skill\(skill\.id\)[\s\S]*?\.equatable\(\)/.test(files.sidebar)
      && /var filteredSkillListResult:[\s\S]*?let issueIndex = SkillListModel\.issueIndex\([\s\S]*?issueCountsBySkillID:\s*issueIndex\.issueCountsBySkillID[\s\S]*?func issueIndicatorCount\(for skill:\s*SkillRecord\) -> Int[\s\S]*?filteredSkillListResult\.issueCount\(for:\s*skill\.id\)/.test(files.storeDerivedState)
      && /struct SkillIssueIndex[\s\S]*?let issueCountsBySkillID:\s*\[SkillRecord\.ID:\s*Int\][\s\S]*?static func issueIndex\([\s\S]*?displayFindings\(skills:\s*skills,\s*findings:\s*findings\)[\s\S]*?sameAgentConflictGroups\(skills:\s*skills,\s*conflicts:\s*conflicts\)[\s\S]*?statusIssueCount/.test(files.storeList)
      && /private struct SkillRow:\s*View,\s*Equatable[\s\S]*?let issueCount:\s*Int[\s\S]*?let isSelected:\s*Bool[\s\S]*?if issueCount > 0[\s\S]*?exclamationmark\.triangle\.fill[\s\S]*?chevron\.right[\s\S]*?\.listPageCardBackground\(isSelected:\s*isSelected\)[\s\S]*?accessibilityAddTraits\(isSelected \? \.isSelected : \[\]\)/.test(files.sidebar)
      && !/private struct SkillRow:[\s\S]*?foregroundStyle\(isSelected \? (?:Color\.)?\.?white/.test(files.sidebar),
  },
  {
    label: "config sidebar exposes scope filtering, clean operation support, disabled skills, and selectable config history",
    text: files.sidebar + "\n" + files.agentConfigWorkspace,
    passed: /var visibleConfigDocuments:[\s\S]*?currentAgentConfigDocuments[\s\S]*?document\.agent == agentFilter\.rawValue[\s\S]*?configScopeFilter\.includes\(document\)[\s\S]*?configDocumentMatchesSidebarQuery\(document\)[\s\S]*?lhs\.scope\.lowercased\(\)\.contains\("project"\)[\s\S]*?localizedStandardCompare/.test(files.store)
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
      && /var selectedConfigDocument:[\s\S]*?case let \.configDocument\(target\)[\s\S]*?currentAgentConfigDocuments\.first[\s\S]*?func selectConfigDocument\(_ document:[\s\S]*?guard selectedSidebarSelection != \.configDocument\(document\.target\)[\s\S]*?selectedSidebarSelection = \.configDocument\(document\.target\)/.test(files.storeSurface)
      && /AgentConfigOverviewDetailPanel\(selectedDocument:\s*store\.selectedConfigDocument\)[\s\S]*?let selectedDocument:[\s\S]*?if let selectedDocument[\s\S]*?currentAgentConfigSection\(documents:\s*\[selectedDocument\]\)/.test(files.agentConfigWorkspace)
      && !/AgentConfigCapabilityCard|AgentConfigDisabledSkillsPanel/.test(files.agentConfigWorkspace)
      && !/Text\(capability\?\.status/.test(files.sidebar + "\n" + files.agentConfigWorkspace),
  },
  {
    label: "current config detail uses the unified single-card editor layout",
    text: files.agentConfigWorkspace + "\n" + files.uiOptimization,
    passed: /static let configEditor = ConfigEditorPresentation\(\)/.test(files.uiOptimization)
      && /struct ConfigEditorPresentation:[\s\S]*?usesSingleCodeCard = true[\s\S]*?showsLineNumbers = true[\s\S]*?usesCompactToolbarActions = true[\s\S]*?primarySaveButtonVisible = false[\s\S]*?autosaveEnabled = true/.test(files.uiOptimization)
      && /private struct ConfigCodeCard<[\s\S]*?PrivacyPathText\(path:\s*path[\s\S]*?toolbar\(\)[\s\S]*?content\(\)[\s\S]*?\.nativePanelSurface\(\)/.test(files.agentConfigWorkspace)
      && /private struct ConfigCodeToolbar:[\s\S]*?UIStrings\.reload[\s\S]*?UIStrings\.formatJSON[\s\S]*?isSensitiveVisible \? "eye\.slash" : "eye"[\s\S]*?onReveal/.test(files.agentConfigWorkspace)
      && /private struct AgentCurrentConfigDocumentsSection:[\s\S]*?ConfigCodeCard\([\s\S]*?title:\s*UIStrings\.currentConfigFile[\s\S]*?path:\s*primaryDocument\?\.target[\s\S]*?statusText:\s*primaryDocument\?\.exists == true \? UIStrings\.existingFile : UIStrings\.willCreateFile[\s\S]*?ConfigCodeToolbar\([\s\S]*?onReload:\s*reload[\s\S]*?JSONSyntaxHighlightedText\(content:\s*displayedContent\)/.test(files.agentConfigWorkspace)
      && !/Label\(UIStrings\.save,\s*systemImage:\s*"square\.and\.arrow\.down"\)/.test(files.agentConfigWorkspace)
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
      && /private func toggleSensitiveEditing\(\)[\s\S]*?if revealsSensitiveConfig \{[\s\S]*?resetDraftFromStore\(\)[\s\S]*?\} else \{[\s\S]*?isConfirmingConfigEdit = true[\s\S]*?\}/.test(files.agentConfigWorkspace)
      && /\.confirmationDialog\(\s*UIStrings\.agentConfigEditConfirmationTitle,[\s\S]*?isPresented:\s*\$isConfirmingConfigEdit[\s\S]*?Button\(UIStrings\.agentConfigShowSensitive,\s*role:\s*\.destructive\)[\s\S]*?revealsSensitiveConfig = true[\s\S]*?Text\(UIStrings\.agentConfigEditConfirmationMessage\)/.test(files.agentConfigWorkspace)
      && /if revealsSensitiveConfig \{[\s\S]*?JSONLineNumberedEditor\(text:\s*displayedDraft\)[\s\S]*?\} else \{[\s\S]*?JSONSyntaxHighlightedText\(content:\s*displayedDraft\.wrappedValue\)/.test(files.agentConfigWorkspace)
      && /private func handleConfigDraftChange\(\)[\s\S]*?ConfigAutosaveDraftReducer\.reduce\([\s\S]*?event:\s*\.userChanged[\s\S]*?case let \.submit\(content,\s*validationError\)[\s\S]*?store\.submitConfigAutosave/.test(files.agentConfigWorkspace)
      && !/private func handleConfigDraftChange\(\)[\s\S]*?Task\.sleep|private func handleConfigDraftChange\(\)[\s\S]*?store\.saveClaudeSettings/.test(files.agentConfigWorkspace)
      && /@Published private\(set\) var configAutosavePhase:\s*RevisionAutosavePhase = \.idle/.test(files.store)
      && /private lazy var configAutosaveCoordinator = RevisionAutosaveCoordinator<ConfigSaveBinding>/.test(files.store)
      && /private struct JSONSyntaxHighlightedText:[\s\S]*?ForEach\(Array\(Self\.lines\(in:\s*content\)\.enumerated\(\)\)[\s\S]*?Text\(Self\.highlighted[\s\S]*?NSRegularExpression[\s\S]*?AttributedString/.test(files.agentConfigWorkspace)
      && /private struct JSONLineNumberedEditor:[\s\S]*?ConfigLineNumberColumn\(lineCount:\s*lineCount\)[\s\S]*?TextEditor\(text:\s*\$text\)/.test(files.agentConfigWorkspace)
      && /static var agentConfigEditConfirmationTitle/.test(files.uiStrings)
      && /static var agentConfigEditConfirmationMessage/.test(files.uiStrings)
      && /static var configAutosavePending/.test(files.uiStrings)
      && /static var configAutosaveSaving/.test(files.uiStrings)
      && /static var formatJSON/.test(files.uiStrings)
      && /"settings\.agentConfig\.editConfirmation\.title"/.test(files.localizable)
      && /"settings\.agentConfig\.editConfirmation\.message"/.test(files.localizableZh)
      && /"settings\.agentConfig\.autosavePending"/.test(files.localizable)
      && /"settings\.agentConfig\.autosaveSaving"/.test(files.localizableZh)
      && /"action\.formatJSON"/.test(files.localizableZh),
  },
  {
    label: "config passive hydration preserves pending work and adopts persisted state after success",
    text: files.agentConfigWorkspace + "\n" + files.store + "\n" + files.revisionAutosave,
    passed: /@Published private\(set\) var configAutosaveDraft:\s*String\?/.test(files.store)
      && /private func hydrateConfigDraftFromStore\([\s\S]*?ConfigAutosaveDraftReducer\.reduce\([\s\S]*?event:\s*\.hydrate\([\s\S]*?storeDraft:\s*store\.configAutosaveDraft[\s\S]*?persistedContent:\s*store\.claudeSettings\?\.content/.test(files.agentConfigWorkspace)
      && /\.onChange\(of:\s*store\.claudeSettings\)[\s\S]*?hydrateConfigDraftFromStore\(revealsSensitive:\s*revealsSensitiveConfig\)/.test(files.agentConfigWorkspace)
      && /\.onChange\(of:\s*store\.configAutosaveDraft\)[\s\S]*?hydrateConfigDraftFromStore\(revealsSensitive:\s*revealsSensitiveConfig\)/.test(files.agentConfigWorkspace)
      && /\.task\(id:\s*store\.selectedAgentConfigRefreshKey\)[\s\S]*?hydrateConfigDraftFromStore\(\)/.test(files.agentConfigWorkspace)
      && !extractFunctionBody(files.agentConfigWorkspace, "hydrateConfigDraftFromStore").includes("cancelPendingConfigAutosave")
      && /enum ConfigAutosaveDraftReducer[\s\S]*?case let \.hydrate\(storeDraft,\s*persistedContent\)[\s\S]*?AutosaveDraftPresentation\.resolve[\s\S]*?action:\s*\.none/.test(files.revisionAutosave)
      && /private func handleConfigDraftChange\([\s\S]*?configAutosaveHasActiveSave[\s\S]*?submitConfigAutosave/.test(files.agentConfigWorkspace)
      && /latestConfigAutosaveRevision/.test(files.store)
      && /handleConfigAutosaveCompletion\([\s\S]*?completion\.revision == latestConfigAutosaveRevision[\s\S]*?configAutosaveDraft = nil/.test(files.store),
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
    label: "detail sections use expanded tag selector",
    text: files.detailSurface,
    pattern: /struct DetailSectionSwitcher:[\s\S]*?ScrollView\(\.horizontal,\s*showsIndicators:\s*false\)[\s\S]*?ForEach\(DetailSection\.visibleCases\)[\s\S]*?DetailSectionTagButton\([\s\S]*?isSelected:\s*selection == item[\s\S]*?selection = item[\s\S]*?private struct DetailSectionTagButton:[\s\S]*?\.background\(background,\s*in:\s*Capsule\(\)\)[\s\S]*?\.accessibilityAddTraits\(isSelected \? \.isSelected : \[\]\)/,
  },
  {
    label: "detail navigation has a stable scroll-to-top anchor",
    text: files.detailSurface,
    pattern: /private static let topAnchorID = "skills-copilot\.detail\.top"[\s\S]*?ScrollViewReader\s*{\s*proxy\s+in[\s\S]*?\.id\(Self\.topAnchorID\)/,
  },
  {
    label: "detail navigation scrolls to top when the selected section changes",
    text: files.detailSurface,
    pattern: /\.onChange\(of:\s*store\.selectedDetailSection\)[\s\S]*?proxy\.scrollTo\(Self\.topAnchorID,\s*anchor:\s*\.top\)/,
  },
  {
    label: "detail sections expose only visible skill detail surfaces while omitting retired work surfaces",
    text: files.detailSurface,
    passed: /static var visibleCases:[\s\S]*?\[\.overview,\s*\.findings,\s*\.history,\s*\.metadata\][\s\S]*?static var primaryWorkCases:[\s\S]*?\[\]/.test(files.detailSection)
      && !/static var visibleCases:[\s\S]*?\.conflicts/.test(files.detailSection)
      && !/static var visibleCases:[\s\S]*?\.analysis/.test(files.detailSection),
    pattern: /static var visibleCases:[\s\S]*?\[\.overview,\s*\.findings,\s*\.history,\s*\.metadata\][\s\S]*?static var primaryWorkCases:[\s\S]*?\[\]/,
  },
  {
    label: "detail router separates session, config, and skill details while modal tools stay outside detail routing",
    text: files.detailSurface,
    passed: /if store\.selectedSidebarSelection\?\.isSession == true[\s\S]*?AgentSessionDetailPanel\(\)[\s\S]*?else if store\.selectedSidebarSelection\?\.isConfig == true[\s\S]*?AgentConfigDetailPanel\(\)[\s\S]*?else if store\.selectedSidebarSelection\?\.isSkill == true,\s*let skill[\s\S]*?SkillDetailContentView\([\s\S]*?else \{[\s\S]*?EmptyDetailView\([\s\S]*?title:\s*emptyDetailTitle[\s\S]*?message:\s*emptyDetailMessage/.test(files.detailSurface)
      && !/isPreflight/.test(files.detailSurface),
  },
  {
    label: "definition hashes stay in metadata instead of overview/header grids",
    text: files.detailOverview + "\n" + files.detailHeaderOverview,
    passed: /private var diagnosticRows:[\s\S]*?CompactMetadataRow\(label:\s*UIStrings\.agent[\s\S]*?CompactMetadataRow\(label:\s*UIStrings\.scope[\s\S]*?CompactMetadataRow\(label:\s*UIStrings\.provenanceKind[\s\S]*?CompactMetadataRow\([\s\S]*?label:\s*UIStrings\.source/.test(files.detailOverview)
      && !/private var diagnosticRows:[\s\S]*?UIStrings\.definition[\s\S]*?\n\s*\]\n\s*\}/.test(files.detailOverview)
      && !/private var headerMetadataRows:[\s\S]*?UIStrings\.definition[\s\S]*?\n\s*\]\n\s*\}/.test(files.detailHeaderOverview)
      && /MetadataRow\(label:\s*UIStrings\.definition,\s*value:\s*skill\.definitionId\)/.test(files.detailHeaderOverview),
  },
  {
    label: "high-priority accessibility and localized summary fixes are present",
    text: files.detailPrimitives + "\n" + files.agentConfigWorkspace + "\n" + files.skillManager + "\n" + files.sidebar + "\n" + files.content + "\n" + files.agentSessionDetail + "\n" + files.uiStrings,
    passed: /struct SummaryChip:[\s\S]*?\.accessibilityElement\(children:\s*\.combine\)[\s\S]*?\.accessibilityLabel\(title\)[\s\S]*?\.accessibilityValue\(value\)/.test(files.detailPrimitives)
      && /private struct AgentConfigAgentIcon:[\s\S]*?\.accessibilityLabel\(filter\.title\)/.test(files.agentConfigWorkspace)
      && /private struct SearchResultRow:[\s\S]*?\.accessibilityElement\(children:\s*\.combine\)[\s\S]*?\.accessibilityLabel\(result\.name\)/.test(files.skillManager)
      && /private struct InstalledSkillRow:[\s\S]*?\.accessibilityElement\(children:\s*\.combine\)[\s\S]*?\.accessibilityLabel\(record\.name\)/.test(files.skillManager)
      && /private struct LocalSkillLibraryRow:[\s\S]*?\.accessibilityElement\(children:\s*\.combine\)[\s\S]*?\.accessibilityLabel\(skill\.name\)/.test(files.skillManager)
      && /private struct SecondarySidebarProjectPickerMenu:[\s\S]*?\.accessibilityLabel\(UIStrings\.text\("project\.chooseMenu"/.test(files.sidebar),
  },
  {
    label: "UIStrings falls back through native localization before defaults",
    text: files.uiStrings,
    pattern: /static func text\(_ key:\s*String,\s*_ defaultValue:\s*String\) -> String[\s\S]*?if let value = localizedStrings\(\)\[key\][\s\S]*?Bundle\.main\.localizedString\(forKey:\s*key,\s*value:\s*nil,\s*table:\s*nil\)[\s\S]*?nativeValue != key[\s\S]*?return defaultValue/,
  },
  {
    label: "agent summary metrics are folded into primary sidebar cards",
    text: files.sidebar,
    passed: /private var sessionCardMetrics:[\s\S]*?scopedLocalSessionUserMessageCount[\s\S]*?scopedLocalSessionTotalMessageCount[\s\S]*?scopedLocalSessionToolCallCount[\s\S]*?scopedLocalSessionSkillCallCount[\s\S]*?private var skillCardMetrics:[\s\S]*?agentEnabledCount[\s\S]*?agentCopilot\.metric\.disabled[\s\S]*?agentDisabledCount[\s\S]*?agentFindingCount[\s\S]*?agentConflictCount[\s\S]*?private var configCardMetrics:[\s\S]*?sidebar\.config\.filesShort[\s\S]*?configDocumentCount[\s\S]*?sidebar\.config\.projectShort[\s\S]*?projectConfigDocumentCount[\s\S]*?sidebar\.config\.historyShort[\s\S]*?configHistoryCount[\s\S]*?private struct SidebarNavigationCardButton:[\s\S]*?if !metrics\.isEmpty[\s\S]*?HStack\(spacing:\s*5\)[\s\S]*?private struct SidebarNavigationMetricPill:/.test(files.sidebar)
      && !/configSupportMetric/.test(files.sidebar)
      && !/private var configCardMetrics:[\s\S]*?sidebar\.config\.disabledShort/.test(files.sidebar)
      && !/configCapability\?\.scan|configCapability\?\.configToggle|configCapability\?\.configSnapshot|configCapability\?\.writable/.test(files.sidebar),
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
    label: "LLM prompt instructions forbid model tables and whole-answer code fences",
    text: `${files.serviceLLM}\n${files.serviceLLMPromptHelpers}`,
    pattern: /Required output: return only valid JSON[\s\S]*?Do not wrap it in Markdown fences[\s\S]*?Required output: concise Markdown draft guidance[\s\S]*?Do not use Markdown tables[\s\S]*?Do not wrap the answer in fenced code blocks/,
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
    pattern: /struct DenseDisclosureList<Item,\s*RowContent:\s*View>:[\s\S]*?visibleLimit:\s*Int = 6[\s\S]*?ForEach\(Array\(items\.prefix\(visibleLimit\)\.enumerated\(\)\),\s*id:\s*\\\.offset\)[\s\S]*?DisclosureGroup\(isExpanded:\s*\$isExpanded\)[\s\S]*?items\.dropFirst\(visibleLimit\)/,
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
    passed: /List\s*\{[\s\S]*?Section\(UIStrings\.text\("sidebar\.primaryNavigation"/.test(files.sidebar)
      && !/ProjectContextControls\(\)/.test(files.sidebar)
      && !/AgentWorkspaceHeader\(\)/.test(files.sidebar)
      && !/private struct AgentWorkspaceHeader/.test(files.sidebar)
      && !/private struct AgentSelectorMenu/.test(files.sidebar)
      && !/WindowChromeAgentControl/.test(files.content)
      && !/WindowChromeProjectControl/.test(files.content)
      && !/WindowChromeTitlebarInstaller|WindowChromeChildWindow|WindowChromeTitlebarLayout/.test(files.content)
      && /SecondarySidebarView\(columnVisibility:\s*columnVisibility\)/.test(files.content)
      && !/secondarySidebarHeaderWidth/.test(files.content)
      && /ZStack\(alignment:\s*\.topTrailing\)[\s\S]*?globalSearchResultsOverlay[\s\S]*?pinnedWindowChromeControls/.test(files.content)
      && /private var pinnedWindowChromeControls:\s*some View\s*\{[\s\S]*?WindowChromeTitlebarAccessory\s*\{[\s\S]*?WindowChromeToolbarControls\([\s\S]*?text:\s*\$globalSearchText,[\s\S]*?isSearchFocused:\s*\$isGlobalSearchFocused,[\s\S]*?showsSearchResults:\s*\$showsGlobalSearchResults,[\s\S]*?onSubmit:\s*selectFirstGlobalSearchResult[\s\S]*?\.frame\(width:\s*0,\s*height:\s*0\)[\s\S]*?\.zIndex\(10\)/.test(files.content)
      && !/ToolbarItem\(placement:\s*\.primaryAction\)\s*\{\s*WindowChromeToolbarControls/.test(files.content)
      && !/ToolbarItem\(placement:\s*\.navigation\)\s*\{\s*WindowChromeToolbarControls/.test(files.content)
      && !/private struct WindowChromeTopBarBackdrop/.test(files.content)
      && /private struct WindowChromeTitlebarAccessory<Content:\s*View>:\s*NSViewRepresentable[\s\S]*?accessory\.layoutAttribute = \.right[\s\S]*?FirstMouseTitlebarAccessoryContainer/.test(files.content)
      && !/WindowChromeTopGlass|windowChromeTopGlass|PassthroughWindowChromeHostingView|topGlassHeight/.test(files.content)
      && /private struct WindowChromeToolbarControls:\s*View[\s\S]*?HStack\(spacing:\s*8\)\s*\{\s*TitlebarAgentSelectorControl\(\)[\s\S]*?\.frame\(width:\s*agentWidth,\s*height:\s*controlHeight,\s*alignment:\s*\.leading\)\s*TitlebarProjectPickerControl\(isCompact:\s*false\)[\s\S]*?\.frame\(width:\s*projectWidth,\s*height:\s*controlHeight,\s*alignment:\s*\.leading\)\s*WindowChromeTrailingControls\([\s\S]*?text:\s*\$text/.test(files.content)
      && !extractStructBody(files.content, "WindowChromeToolbarControls").includes("Divider()")
      && !/\.toolbar\s*\{[\s\S]*?ToolbarItem\(placement:\s*\.navigation\)[\s\S]*?TitlebarAgentSelectorControl\(\)/.test(files.content)
      && /struct SecondarySidebarView:[\s\S]*?let columnVisibility:\s*NavigationSplitViewVisibility[\s\S]*?List\(selection:\s*\$store\.selectedSidebarSelection\)[\s\S]*?\.padding\(\.top,\s*50\)[\s\S]*?GeometryReader \{ proxy in[\s\S]*?SecondarySidebarHeaderWidthPreferenceKey\.self[\s\S]*?\.allowsHitTesting\(false\)/.test(files.sidebar)
      && !/\.overlay\(alignment:\s*\.topLeading\)[\s\S]*?SecondarySidebarHeaderChrome/.test(files.sidebar)
      && /private struct TitlebarAgentSelectorControl:\s*View[\s\S]*?isPopoverPresented\.toggle\(\)[\s\S]*?TitlebarAgentSelectorLabel\([\s\S]*?\.popover\(isPresented:\s*\$isPopoverPresented[\s\S]*?ForEach\(SkillAgentFilter\.managementCases\)[\s\S]*?store\.agentFilter = filter[\s\S]*?\.accessibilityValue\(store\.agentFilter\.title\)/.test(files.content)
      && /private struct TitlebarAgentIconBadge:[\s\S]*?var size:\s*CGFloat = 28[\s\S]*?frame\(width:\s*imageSize,\s*height:\s*imageSize\)[\s\S]*?frame\(width:\s*size,\s*height:\s*size\)/.test(files.content)
      && /private struct TitlebarProjectPickerControl:\s*View[\s\S]*?Button\s*\{[\s\S]*?isPopoverPresented\.toggle\(\)[\s\S]*?\.popover\(isPresented:\s*\$isPopoverPresented[\s\S]*?ForEach\(store\.recentProjectContexts\)[\s\S]*?await store\.setProject\([\s\S]*?NSOpenPanel\(\)[\s\S]*?NSWorkspace\.shared\.activateFileViewerSelecting/.test(files.content)
      && !/return 224/.test(files.content)
      && !/SecondarySidebarProjectPickerMenu\(isCompact:\s*true\)[\s\S]*?frame\(maxWidth:\s*\.infinity,\s*alignment:\s*\.trailing\)/.test(files.sidebar)
      && !/ProjectContextToolbarControl/.test(files.sidebar)
      && !/store\.selectedSidebarSelection\s*=\s*\.agentWorkspace/.test(files.sidebar)
      && !/\.tag\(SidebarSelection\.agentWorkspace\)/.test(files.sidebar),
  },
  {
    label: "secondary sidebar project menu owns merged project selection and actions",
    text: files.sidebar,
    pattern: /private struct SecondarySidebarProjectPickerMenu:[\s\S]*?Menu\s*\{[\s\S]*?Label\(UIStrings\.chooseProject,\s*systemImage:\s*"folder\.badge\.plus"\)[\s\S]*?Section\(UIStrings\.recentProjects\)[\s\S]*?await store\.setProject\([\s\S]*?Label\(UIStrings\.revealInFinder,[\s\S]*?arrow\.up\.forward\.app[\s\S]*?Label\(UIStrings\.clearProject,[\s\S]*?xmark\.circle[\s\S]*?SecondarySidebarProjectPickerLabel\([\s\S]*?\.menuStyle\(\.button\)[\s\S]*?\.buttonStyle\(\.plain\)[\s\S]*?private struct SecondarySidebarProjectPickerLabel:[\s\S]*?ViewThatFits\(in:\s*\.horizontal\)/,
  },
  {
    label: "skill-list batch action lives in the compact toolbar",
    text: files.sidebar,
    passed: /private func skillToolbar\(visibleSkills:[\s\S]*?searchField[\s\S]*?batchToolbarButton\(visibleSkills:\s*visibleSkills\)/.test(files.sidebar)
      && /private func batchToolbarButton\(visibleSkills:[\s\S]*?resetBatchToggleSelectionToVisibleSkills\(\)[\s\S]*?isBatchOperationPresented = true[\s\S]*?Image\(systemName:\s*"checklist\.checked"\)/.test(files.sidebar)
      && /private struct SkillListSectionHeader:[\s\S]*?Text\(UIStrings\.batchToggleSelectedCount\(visibleCount\)\)/.test(files.sidebar)
      && !/private struct SkillListSectionHeader:[\s\S]*?Button\(action:\s*action\)/.test(files.sidebar),
  },
  {
    label: "findings expose only the rule filter in the control panel",
    passed: /Picker\(UIStrings\.findingRuleFilter,\s*selection:\s*\$ruleFilter\)/.test(files.detailSurface)
      && /rulePicker\.frame\(width:\s*250\)/.test(files.detailSurface)
      && !/Picker\(UIStrings\.findingSeverityFilter,\s*selection:\s*\$severityFilter\)/.test(files.detailSurface)
      && !/FindingsSummaryOverview/.test(files.detailSurface)
      && !/FindingsSummaryStrip/.test(files.detailSurface),
  },
  {
    label: "findings render severity groups",
    text: files.detailSurface,
    pattern: /FindingSeverityHeader\(group:\s*group\)/,
  },
  {
    label: "findings render remediation guidance",
    text: files.detailSurface,
    pattern: /Label\(UIStrings\.findingRemediation,\s*systemImage:\s*"wrench\.and\.screwdriver"\)/,
  },
  {
    label: "detail renders permissions without safety verdicts",
    text: files.detailSurface,
    pattern: /PermissionSummaryCard\(summary:\s*PermissionDisplayModel\.summary\(for:\s*detail\.permissions\)\)/,
  },
  {
    label: "overview hides placeholder permission and script risk noise",
    text: files.detailOverview,
    passed: /if showsOverviewRiskPanel\s*\{[\s\S]*?OverviewRiskPanel\(/.test(files.detailOverview)
      && /private var showsOverviewRiskPanel:[\s\S]*?PermissionDisplayModel\.hasOverviewSignal\(for:\s*permissionPayload\)[\s\S]*?scriptPreview\?\.hasOverviewSignal == true/.test(files.detailOverview),
  },
  {
    label: "snapshot preview sheet has bounded width",
    text: files.detailSurface,
    pattern: /\.frame\(width:\s*980,\s*height:\s*680\)/,
  },
  {
    label: "snapshot preview panes are scrollable for long content",
    text: files.detailSurface,
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
    passed: /private enum SettingsTab:[\s\S]*?CaseIterable[\s\S]*?case appearance[\s\S]*?case provider[\s\S]*?case providerObservability[\s\S]*?case service/.test(files.settings)
      && /HStack\(spacing:\s*0\)[\s\S]*?settingsSidebar[\s\S]*?Divider\(\)[\s\S]*?selectedSettingsPane/.test(files.settings)
      && /private var settingsSidebar:[\s\S]*?ForEach\(SettingsTab\.allCases\)[\s\S]*?SettingsSidebarItem/.test(files.settings)
      && /private var selectedSettingsPane:[\s\S]*?switch selectedSettingsTab[\s\S]*?case \.providerObservability:[\s\S]*?ProviderObservabilitySettingsPanel\(\)/.test(files.settings)
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
      && /SettingsPageHeader\([\s\S]*?title:\s*UIStrings\.service/.test(files.settings)
      && /DetailMetricGrid\(maxColumns:\s*3/.test(files.settings)
      && /Picker\(UIStrings\.themeSelection,[\s\S]*?ForEach\(AppTheme\.allCases\)[\s\S]*?\.pickerStyle\(\.segmented\)[\s\S]*?\.labelsHidden\(\)/.test(files.settings)
      && /Picker\(UIStrings\.languageSelection,[\s\S]*?\.pickerStyle\(\.segmented\)[\s\S]*?\.labelsHidden\(\)/.test(files.settings)
      && /Picker\(UIStrings\.llmProvider,[\s\S]*?\.pickerStyle\(\.segmented\)[\s\S]*?\.labelsHidden\(\)/.test(files.settings),
  },
  {
    label: "settings AI provider autosaves profile edits while confirming provider tests",
    text: files.settings + "\n" + files.store + "\n" + files.uiStrings + "\n" + files.localizable + "\n" + files.localizableZh,
    passed: !/@State private var providerAutosaveTask: Task<Void,\s*Never>\?/.test(files.settings)
      && /@State private var isConfirmingProviderTest = false/.test(files.settings)
      && /\.onChange\(of:\s*providerDraft\)[\s\S]*?handleProviderDraftChange\(\)/.test(files.settings)
      && /\.onChange\(of:\s*store\.providerAutosaveDraft\)[\s\S]*?AutosaveDraftPresentation\.resolve[\s\S]*?providerDraft = resolvedDraft/.test(files.settings)
      && /\.task\(id:\s*selectedSettingsTab\)[\s\S]*?case \.provider:[\s\S]*?hydrateProviderDraftFromStore\(\)/.test(files.settings)
      && /private func hydrateProviderDraftFromStore\(\)[\s\S]*?AutosaveDraftPresentation\.resolve\([\s\S]*?storeDraft:\s*store\.providerAutosaveDraft[\s\S]*?persistedValue:\s*AIProviderSettingsDraft\(status:\s*store\.aiProviderStatus\)/.test(files.settings)
      && !extractFunctionBody(files.settings, "hydrateProviderDraftFromStore").includes("cancelPendingProviderAutosave")
      && /private func handleProviderDraftChange\(\)[\s\S]*?store\.providerAutosaveDraft != providerDraft[\s\S]*?providerAutosaveHasActiveSave[\s\S]*?store\.submitProviderAutosave\(draft:\s*providerDraft\)/.test(files.settings)
      && !/private func handleProviderDraftChange\(\)[\s\S]*?Task\.sleep|private func handleProviderDraftChange\(\)[\s\S]*?store\.saveAIProviderSettings/.test(files.settings)
      && /@Published private\(set\) var providerAutosavePhase:\s*RevisionAutosavePhase = \.idle/.test(files.store)
      && /private lazy var providerAutosaveCoordinator = RevisionAutosaveCoordinator<AIProviderSettingsDraft>/.test(files.store)
      && /private func handleProviderAutosaveCompletion\([\s\S]*?completion\.revision == latestProviderAutosaveRevision[\s\S]*?providerAutosaveDraft = nil/.test(files.store)
      && /UIStrings\.aiProviderAutosavePending/.test(files.settings)
      && /Button\s*\{[\s\S]*?isConfirmingProviderTest = true[\s\S]*?\} label:\s*\{[\s\S]*?Label\(UIStrings\.aiProviderTest,\s*systemImage:\s*"network"\)/.test(files.settings)
      && /\.confirmationDialog\(\s*UIStrings\.aiProviderTestConfirmationTitle,[\s\S]*?isPresented:\s*\$isConfirmingProviderTest[\s\S]*?Button\(UIStrings\.aiProviderTest,\s*role:\s*\.destructive\)[\s\S]*?testProviderConnection\(\)[\s\S]*?Text\(UIStrings\.aiProviderTestConfirmationMessage\)/.test(files.settings)
      && /private func testProviderConnection\(\)[\s\S]*?await store\.testAIProviderConnection\(draft:\s*providerDraft\)/.test(files.settings)
      && !/isConfirmingProviderSave/.test(files.settings)
      && !/Label\(UIStrings\.aiProviderSave,\s*systemImage:\s*"square\.and\.arrow\.down"\)/.test(files.settings)
      && /static var aiProviderAutosavePending/.test(files.uiStrings)
      && /static var aiProviderTestConfirmationMessage/.test(files.uiStrings)
      && /"settings\.aiProvider\.autosavePending"/.test(files.localizable)
      && /"settings\.aiProvider\.testConfirmation\.message"/.test(files.localizableZh),
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
    label: "detail uses privacy path rows for high-risk paths",
    text: files.detailSurface,
    pattern: /PrivacyPathRow\(label:\s*UIStrings\.source,\s*path:\s*skill\.displayPath\)[\s\S]*?PrivacyPathRow\(label:\s*UIStrings\.source,\s*path:\s*preview\.sourcePath\)/,
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
    label: "tool-global preview uses read-only install affordance",
    text: files.detailSurface,
    pattern: /ToolGlobalPreviewCard\(skill:\s*skill\)/,
  },
  {
    label: "tool-global install confirmation uses verified write copy",
    text: files.detailSurface,
    pattern: /store\.confirmToolInstall\(skill:\s*skill,\s*target:\s*preview\.target\)/,
  },
  {
    label: "sidebar labels read-only preview rows",
    text: files.sidebar,
    pattern: /UIStrings\.readOnlyPreview/,
  },
  {
    label: "localized LLM action labels are present",
    text: files.localizable,
    pattern: /"llm\.action\.analyze".*"llm\.action\.recommend".*"llm\.action\.explainConflict".*"llm\.action\.draftFrontmatter"/s,
  },
  {
    label: "localized tool-global preview labels are present",
    text: files.localizable,
    pattern: /"detail\.toolGlobal\.previewTitle".*"detail\.toolGlobal\.installReady".*"detail\.toolGlobal\.installConfirmation"/s,
  },
  {
    label: "localized finding filter labels are present",
    text: files.localizable,
    pattern: /"findings\.filter\.rule".*"findings\.filter\.allRules"/s,
  },
  {
    label: "localized adapter capability labels are present",
    text: files.localizable,
    pattern: /"sidebar\.adapterCapabilities".*"adapter\.capability\.scan".*"adapter\.capability\.toggle".*"adapter\.capability\.install"/s,
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
      "taskCockpit.history.summary",
      "taskCockpit.action.build",
      "taskCockpit.empty.result",
      "taskCockpit.recommendedSkill",
    ].every((key) => files.localizable.includes(`"${key}" =`)),
  },
  {
    label: "localized skill manager workflow and unavailable-tool labels are present",
    text: files.localizable,
    pattern: /"skillManager\.workflow\.accessibility".*"skillManager\.workflow\.searchInstall".*"skillManager\.workflow\.installedUpdates".*"skillManager\.workflow\.localLibrary".*"skillManager\.toolUnavailable\.title".*"skillManager\.toolUnavailable\.message"/s,
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
    label: "secondary sidebar rows use muted selection or white card treatment",
    passed: ["ConfigCurrentDocumentSidebarRow", "ConfigSnapshotSidebarRow"].every((name) => {
      const body = extractStructBody(files.sidebar, name);
      return body.includes(".optimizedSidebarSelection(isSelected: isSelected)")
        && !/foregroundStyle\(isSelected \? (?:Color\.)?\.?white/.test(body)
        && !/fill\(Color\.accentColor\)/.test(body);
    })
      && ["SessionSidebarRow", "SkillRow"].every((name) => {
        const body = extractStructBody(files.sidebar, name);
        return body.includes(".listPageCardBackground(isSelected: isSelected)")
          && body.includes("minimumCardRowHeight")
          && !/foregroundStyle\(isSelected \? (?:Color\.)?\.?white/.test(body);
      })
      && /struct SidebarSelectionPresentation:[\s\S]*?usesSaturatedAccentBackground = false[\s\S]*?usesWhiteSelectedText = false[\s\S]*?accentLineWidth = 3/.test(files.uiOptimization)
      && /private struct ListPageCardBackgroundModifier:[\s\S]*?selectedContentBackgroundColor/.test(files.sidebar)
      && /private struct ListPageCardBackgroundModifier:[\s\S]*?UIOptimizationPresentation\.sidebarSelection\.accentLineWidth/.test(files.sidebar)
      && /private struct OptimizedSidebarSelectionModifier:[\s\S]*?selectedContentBackgroundColor[\s\S]*?UIOptimizationPresentation\.sidebarSelection\.accentLineWidth/.test(files.sidebar),
  },
  {
    label: "session and config sidebars use compact search plus icon refresh toolbars",
    passed: /struct SidebarSecondaryListPresentation:[\s\S]*?minimumSearchWidth = 220[\s\S]*?compactRowMinHeight = 40[\s\S]*?compactRowMaxHeight = 44[\s\S]*?refreshUsesIconOnly = true/.test(files.uiOptimization)
      && /private struct SidebarSearchField:[\s\S]*?TextField\(placeholder,\s*text:\s*\$text\)[\s\S]*?\.textFieldStyle\(\.roundedBorder\)[\s\S]*?\.controlSize\(\.small\)[\s\S]*?\.frame\(minWidth:\s*minimumWidth,\s*maxWidth:\s*\.infinity\)/.test(files.sidebar)
      && /private var sessionToolbar:[\s\S]*?let layout = UIOptimizationPresentation\.skillList[\s\S]*?VStack\(alignment:\s*\.leading,\s*spacing:\s*8\)[\s\S]*?ListPageTitleBlock\([\s\S]*?HStack\(alignment:\s*\.center,\s*spacing:\s*CGFloat\(layout\.filterControlSpacing\)\)[\s\S]*?sessionScopePicker[\s\S]*?sessionSortPicker[\s\S]*?sessionSortDirectionButton[\s\S]*?sessionRefreshButton[\s\S]*?sessionSearchField/.test(files.sidebar)
      && /private var sessionScopePicker:[\s\S]*?SkillFilterMenuPicker\([\s\S]*?title:\s*UIStrings\.scope[\s\S]*?selection:\s*\$store\.localSessionScopeFilter[\s\S]*?options:\s*LocalSessionScopeFilter\.allCases[\s\S]*?expands:\s*false/.test(files.sidebar)
      && /private var sessionSortPicker:[\s\S]*?SkillFilterMenuPicker\([\s\S]*?title:\s*UIStrings\.sort[\s\S]*?selection:\s*\$store\.localSessionSortOrder[\s\S]*?options:\s*LocalSessionSortOrder\.allCases/.test(files.sidebar)
      && /private func sessionSortDirectionButton\(width:\s*CGFloat,\s*height:\s*CGFloat\)[\s\S]*?store\.localSessionSortDirection = store\.localSessionSortDirection == \.ascending \? \.descending : \.ascending[\s\S]*?Image\(systemName:\s*store\.localSessionSortDirection == \.ascending \? "arrow\.up" : "arrow\.down"\)/.test(files.sidebar)
      && /enum LocalSessionSortOrder:[\s\S]*?case recent[\s\S]*?case title/.test(files.storeList)
      && /@Published var localSessionSortOrder:\s*LocalSessionSortOrder = \.recent/.test(files.store)
      && /@Published var localSessionSortDirection:\s*SkillSortDirection = \.descending/.test(files.store)
      && /func projectedRows\([\s\S]*?criteria:\s*LocalSessionProjectionCriteria[\s\S]*?case \.recent:[\s\S]*?endedAt[\s\S]*?case \.title:[\s\S]*?localizedCaseInsensitiveCompare/.test(files.localSessionCache)
      && !/SessionScopeToggle/.test(files.sidebar)
      && /private var sessionRefreshButton:[\s\S]*?Image\(systemName:\s*"arrow\.clockwise"\)[\s\S]*?\.accessibilityLabel\(UIStrings\.text\("sidebar\.sessions\.preview"/.test(files.sidebar)
      && /private var configToolbar:[\s\S]*?VStack\(alignment:\s*\.leading,\s*spacing:\s*8\)[\s\S]*?HStack\(alignment:\s*\.center,\s*spacing:\s*CGFloat\(layout\.filterControlSpacing\)\)[\s\S]*?configScopePicker[\s\S]*?configRefreshButton\([\s\S]*?configSearchField/.test(files.sidebar)
      && /private func configRefreshButton\(width:\s*CGFloat,\s*height:\s*CGFloat\)[\s\S]*?Image\(systemName:\s*"arrow\.clockwise"\)[\s\S]*?\.accessibilityLabel\(UIStrings\.reload\)/.test(files.sidebar),
  },
  {
    label: "session detail copy and expand actions appear only on row hover",
    passed: /private struct LocalSessionContentItemRow:[\s\S]*?@State private var isHoveringActions = false/.test(files.agentSessionDetail)
      && /private var actionOpacity:\s*Double[\s\S]*?isHoveringActions \? 1 : 0/.test(files.agentSessionDetail)
      && /HStack\(spacing:\s*4\)[\s\S]*?detailButton[\s\S]*?copyButton[\s\S]*?\.opacity\(actionOpacity\)[\s\S]*?\.allowsHitTesting\(isHoveringActions\)/.test(files.agentSessionDetail)
      && /\.onHover\s*\{\s*isHovering in[\s\S]*?isHoveringActions = isHovering/.test(files.agentSessionDetail)
      && /contextMenu[\s\S]*?copyToPasteboard\(item\.text\)[\s\S]*?isShowingFullText = true/.test(files.agentSessionDetail),
  },
  {
    label: "session detail chips use semantic colors for user agent tool and skill content",
    passed: /private func filterLabel\([\s\S]*?tint:\s*Color[\s\S]*?Text\("\\\(count\)"\)[\s\S]*?\.foregroundStyle\(isSelected \? tint : \.secondary\)[\s\S]*?\.background\(\s*isSelected \? tint\.opacity\(0\.16\)/.test(files.agentSessionDetail)
      && /private extension LocalSessionContentKind[\s\S]*?var semanticTint:\s*Color[\s\S]*?case \.userMessage:[\s\S]*?return \.blue[\s\S]*?case \.agentReply:[\s\S]*?return \.purple[\s\S]*?case \.toolCall:[\s\S]*?return \.orange[\s\S]*?case \.skillCall:[\s\S]*?return \.green/.test(files.agentSessionDetail)
      && /Label\(item\.title\.isEmpty \? item\.kind\.title : item\.title,\s*systemImage:\s*item\.kind\.systemImage\)[\s\S]*?\.foregroundStyle\(item\.kind\.semanticTint\)/.test(files.agentSessionDetail),
  },
  {
    label: "detail feedback renders inline and success messages auto-dismiss",
    passed: /struct DetailFeedbackPresentation:[\s\S]*?usesOverlayToast = false[\s\S]*?maximumWidth = 420/.test(files.uiOptimization)
      && /ScrollViewReader[\s\S]*?VStack\(alignment:\s*\.leading,\s*spacing:\s*24\)[\s\S]*?DetailFeedbackInlineView\([\s\S]*?errorMessage:\s*store\.errorMessage,[\s\S]*?lastMutationMessage:\s*store\.lastMutationMessage[\s\S]*?if store\.selectedSidebarSelection/.test(files.detail)
      && /private struct DetailFeedbackInlineView:\s*View,\s*Equatable[\s\S]*?let errorMessage:\s*String\?[\s\S]*?let lastMutationMessage:\s*String\?[\s\S]*?DetailFeedbackToast\([\s\S]*?DetailFeedbackToast\(/.test(files.detail)
      && /struct DetailFeedbackToast:[\s\S]*?UIOptimizationPresentation\.detailFeedback\.maximumWidth[\s\S]*?Color\.agentCopilotPanelBackground/.test(files.detailPrimitives)
      && /@Published private\(set\) var lastMutationMessage:\s*String\?\s*\{[\s\S]*?scheduleLastMutationMessageDismissal\(\)/.test(files.store)
      && /lastMutationMessageDismissTask:[\s\S]*?Task<Void,\s*Never>\?/.test(files.store)
      && /private func scheduleLastMutationMessageDismissal\(\)[\s\S]*?Task\.sleep\(nanoseconds:[\s\S]*?clearLastMutationMessageIfCurrent/.test(files.store)
      && /@Published var errorMessage:\s*String\?\s*\{[\s\S]*?scheduleErrorMessageDismissal\(\)/.test(files.store)
      && /private func scheduleErrorMessageDismissal\(\)[\s\S]*?Task\.sleep\(nanoseconds:[\s\S]*?clearErrorMessageIfCurrent/.test(files.store)
      && !/ZStack\(alignment:\s*\.topTrailing\)[\s\S]*?detailFeedbackOverlay/.test(files.detail)
      && !/allowsHitTesting\(false\)[\s\S]*?DetailFeedbackToast/.test(files.detail),
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
      && /AgentConfigDetailPanel\(\)/.test(files.detail)
      && /struct AgentConfigOverviewDetailPanel/.test(files.agentConfigWorkspace)
      && /struct AgentConfigSnapshotDetailPanel/.test(files.agentConfigWorkspace)
      && !/private struct AgentConfigSnapshotDetailPanel[\s\S]*?DetailMetricGrid[\s\S]*?SummaryChip\(title:\s*UIStrings\.agent/.test(files.agentConfigWorkspace),
  },
  {
    label: "Agent Workspace does not expose the retired evidence surface navigation grid",
    passed: !/AgentProfileNavigationGrid|agentCopilot\.evidenceSurfaces|selectedSidebarSelection\s*=\s*\.work\(section\)/.test(files.agentSessionDetail),
  },
  {
    label: "modal workflows share liquid-glass sheet chrome, columns, and inline feedback",
    passed: /static let workflowSheet = WorkflowSheetPresentation\(\)/.test(files.uiOptimization)
      && /struct WorkflowSheetPresentation:[\s\S]*?titlebarStyle = WorkflowSheetTitlebarStyle\.liquidGlass[\s\S]*?closeActionPlacement = WorkflowSheetCloseActionPlacement\.trailingTitlebar[\s\S]*?feedbackStyle = WorkflowSheetFeedbackStyle\.inlineTintedBanner[\s\S]*?columnLayout = WorkflowSheetColumnLayout\.twoColumn/.test(files.uiOptimization)
      && /struct WorkflowSheetShell<Content: View>:[\s\S]*?Label\(title,\s*systemImage:\s*systemImage\)[\s\S]*?Button\s*\{[\s\S]*?dismiss\(\)[\s\S]*?\} label:\s*\{[\s\S]*?Label\(UIStrings\.done,\s*systemImage:\s*"xmark"\)[\s\S]*?\.background\(\.bar\)/.test(files.workflowSheet)
      && /struct WorkflowSheetSplitLayout<Primary: View,\s*Secondary: View>:[\s\S]*?primary\(\)[\s\S]*?Divider\(\)[\s\S]*?secondary\(\)/.test(files.workflowSheet)
      && /struct WorkflowSheetInlineBanner:[\s\S]*?Label\(message,\s*systemImage:\s*style\.systemImage\)[\s\S]*?\.background\(style\.color\.opacity\(0\.08\)[\s\S]*?Rectangle\(\)[\s\S]*?\.fill\(style\.color\)/.test(files.workflowSheet)
      && /struct TaskPreflightPreviewSheet:[\s\S]*?WorkflowSheetShell\([\s\S]*?WorkflowSheetSplitLayout\([\s\S]*?TaskPreflightEditorPane[\s\S]*?TaskPreflightHistoryPanel/.test(files.taskCockpit)
      && /struct SkillPackageManagerSheet:[\s\S]*?WorkflowSheetShell\([\s\S]*?SkillManagerPanel\(showsHeader:\s*false\)/.test(files.sidebar)
      && /struct SkillManagerPanel:[\s\S]*?WorkflowSheetSplitLayout\(primaryMinWidth:\s*430,\s*secondaryWidth:\s*360\)[\s\S]*?workflowInputContent[\s\S]*?workflowResultsContent/.test(files.skillManager)
      && !/struct SkillPackageManagerSheet:[\s\S]*?ErrorBanner\(message:\s*error\)/.test(files.sidebar)
      && !/struct SkillPackageManagerSheet:[\s\S]*?SuccessBanner\(message:\s*message\)/.test(files.sidebar),
  },
  {
    label: "task preflight opens from the fixed sidebar footer sheet and keeps selectable history",
    passed: /TaskPreflightPreviewSheet\(\)/.test(files.sidebar)
      && /struct TaskPreflightPreviewSheet:[\s\S]*?WorkflowSheetShell\([\s\S]*?WorkflowSheetSplitLayout\([\s\S]*?TaskPreflightHistoryPanel/.test(files.taskCockpit)
      && /TaskPreflightEditorPane:[\s\S]*?TaskCockpitPanel\(/.test(files.taskCockpit)
      && /providerGateMessage/.test(files.taskCockpit)
      && /WorkflowSheetInlineBanner\(message:\s*providerGateMessage,\s*style:\s*\.warning\)/.test(files.taskCockpit)
      && /\.disabled\([\s\S]*?providerGateMessage != nil[\s\S]*?\)/.test(files.taskCockpit)
      && /\.help\(providerGateMessage \?\? UIStrings\.taskCockpitBoundary\)/.test(files.taskCockpit)
      && /private struct TaskCockpitAgentChip:[\s\S]*?\.frame\(minHeight:\s*44,\s*alignment:\s*\.leading\)[\s\S]*?\.frame\(maxWidth:\s*\.infinity,\s*alignment:\s*\.leading\)/.test(files.taskCockpit)
      && !/private struct TaskCockpitAgentChip:[\s\S]*?fixedAgentChipWidth/.test(files.taskCockpit)
      && /taskCockpit\.history\.summary/.test(files.taskCockpit + "\n" + files.localizable)
      && /taskCockpitHistory/.test(files.taskCockpit + "\n" + files.store)
      && /selectTaskCockpitHistoryRecord/.test(files.taskCockpit + "\n" + files.store)
      && /recordTaskCockpitHistory/.test(files.store)
      && !/case preflight/.test(files.sidebarSelection)
      && !/selectedSidebarSelection\s*=\s*\.preflight/.test(files.sidebar + "\n" + files.storeSurface),
  },
  {
    label: "task preflight history empty state uses the shared empty-state component",
    passed: /private struct TaskPreflightHistoryPanel:[\s\S]*?if records\.isEmpty \{[\s\S]*?EmptyState\([\s\S]*?title:\s*UIStrings\.text\("taskCockpit\.history\.emptyTitle"[\s\S]*?systemImage:\s*"clock\.badge\.questionmark"[\s\S]*?message:\s*UIStrings\.text\("taskCockpit\.history\.emptyMessage"/.test(files.taskCockpit)
      && !/Text\(UIStrings\.text\("taskCockpit\.history\.empty"/.test(files.taskCockpit),
  },
  {
    label: "task preflight history is session-only with redacted cleanup retry",
    passed: /TaskPreflightHistoryPanel\([\s\S]*?cleanupMessage:\s*store\.taskCockpitHistoryCleanupMessage[\s\S]*?onClear:\s*\{[\s\S]*?store\.clearTaskCockpitHistory\(\)/.test(files.taskCockpit)
      && /private struct TaskPreflightHistoryPanel:[\s\S]*?@State private var isConfirmingClear = false[\s\S]*?Text\(UIStrings\.taskCockpitHistorySummary\)/.test(files.taskCockpit)
      && /if let cleanupMessage[\s\S]*?WorkflowSheetInlineBanner\(message:\s*cleanupMessage,\s*style:\s*\.warning\)/.test(files.taskCockpit)
      && /Label\(UIStrings\.taskCockpitHistoryClear,\s*systemImage:\s*"trash"\)[\s\S]*?\.disabled\(records\.isEmpty && cleanupMessage == nil\)/.test(files.taskCockpit)
      && /\.confirmationDialog\([\s\S]*?UIStrings\.taskCockpitHistoryClearConfirmationTitle[\s\S]*?Button\(UIStrings\.taskCockpitHistoryClear,\s*role:\s*\.destructive\)[\s\S]*?onClear\(\)[\s\S]*?UIStrings\.taskCockpitHistoryClearConfirmationMessage/.test(files.taskCockpit)
      && /@Published private\(set\) var taskCockpitHistoryCleanupMessage:\s*String\?/.test(files.store)
      && /"taskCockpit\.history\.summary" = "Completed Preflights stay in memory for this app session\. Task text and provider results are not saved to disk and disappear when the app quits\."/.test(files.localizable)
      && /"taskCockpit\.history\.summary" = ".*本次应用会话.*不会保存到磁盘.*退出应用.*"/.test(files.localizableZh)
      && [
        "taskCockpit.history.clear",
        "taskCockpit.history.clearConfirmation.title",
        "taskCockpit.history.clearConfirmation.message",
        "taskCockpit.history.cleanupFailed",
      ].every((key) => files.localizable.includes(`"${key}" =`) && files.localizableZh.includes(`"${key}" =`)),
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
      && /private static func isReviewOnlyRisk\(_ row: TaskCockpitContextRow\) -> Bool[\s\S]*?signalTokens\(for:\s*row\)/.test(files.taskCockpit)
      && /private static func isInternalBoundary\(_ row: TaskCockpitContextRow\) -> Bool[\s\S]*?signalTokens\(for:\s*row\)/.test(files.taskCockpit)
      && /private static func signalTokens\(for row: TaskCockpitContextRow\) -> Set<String>/.test(files.taskCockpit)
      && /private static func normalizedSignalToken\(_ value: String\) -> String/.test(files.taskCockpit)
      && !/normalized\.contains\("task readiness is blocked"\)/.test(files.taskCockpit)
      && !/normalized\.contains\("routing confidence is blocked"\)/.test(files.taskCockpit)
      && !/normalized\.contains\("small score margin"\)/.test(files.taskCockpit)
      && !/normalized\.contains\("close or overlapping alternatives"\)/.test(files.taskCockpit)
      && !/normalized\.contains\("read-only"\)/.test(files.taskCockpit)
      && !/normalized\.contains\("provider not sent"\)/.test(files.taskCockpit)
      && !/normalized\.contains\("task cockpit combined"\)/.test(files.taskCockpit),
  },
  {
    label: "skill filter controls show their role alongside the current value",
    passed: /private struct SkillFilterMenuPicker[\s\S]*?SidebarMenuButtonLabel\([\s\S]*?title:\s*title,[\s\S]*?value:\s*optionTitle\(selection\)/.test(files.sidebar)
      && /private struct SidebarMenuButtonLabel[\s\S]*?Text\(title\)[\s\S]*?Text\(value\)/.test(files.sidebar)
      && /private struct SkillFilterMenuPicker[\s\S]*?\.accessibilityLabel\(title\)[\s\S]*?\.accessibilityValue\(optionTitle\(selection\)\)/.test(files.sidebar)
      && /SkillFilterMenuPicker\([\s\S]*?title:\s*UIStrings\.text\("sidebar\.skillFilter",\s*"Filter"\)/.test(files.sidebar)
      && /SkillFilterMenuPicker\([\s\S]*?title:\s*UIStrings\.text\("sidebar\.scopeFilter",\s*"Scope"\)/.test(files.sidebar)
      && /SkillFilterMenuPicker\([\s\S]*?title:\s*UIStrings\.sort/.test(files.sidebar),
  },
  {
    label: "detail adopting-agent summary uses store-derived cache instead of scanning skills in body",
    passed: /private\(set\) var adoptingAgentSummaryBySkillID: \[SkillRecord\.ID: String\] = \[:\]/.test(files.store)
      && /didSet\s*{[\s\S]*?invalidateFilteredSkillListCache\(\)[\s\S]*?invalidateAdoptingAgentSummaryCache\(\)[\s\S]*?}/.test(files.store)
      && /func ensureAdoptingAgentSummaryCache\(\)[\s\S]*?SkillListModel\.adoptingAgentSummaryBySkillID\(for:\s*skills\)[\s\S]*?isAdoptingAgentSummaryCacheValid = true/.test(files.store)
      && /adoptingAgentSummary:\s*store\.adoptingAgentSummary\(for:\s*skill\)/.test(files.detail)
      && /func adoptingAgentSummary\(for skill: SkillRecord\) -> String/.test(files.storeDerivedState)
      && !/private func adoptingAgentSummary\(for skill: SkillRecord\)[\s\S]*?store\.skills[\s\S]*?\.filter/.test(files.detail),
  },
  {
    label: "native panel surface uses shared white presentation corner radius",
    passed: /static let surfaceCornerRadius = sidebarSelection\.rowCornerRadius/.test(files.uiOptimization)
      && /RoundedRectangle\(cornerRadius:\s*CGFloat\(UIOptimizationPresentation\.surfaceCornerRadius\)\)/.test(files.nativePanelSurface)
      && !/RoundedRectangle\(cornerRadius:\s*8\)/.test(files.nativePanelSurface),
  },
  {
    label: "Skill Manager uses segmented workflows, local feedback, and unavailable-tool gating",
    passed: /enum SkillManagerWorkflow:[\s\S]*?case searchInstall[\s\S]*?case installedUpdates[\s\S]*?case localLibrary/.test(files.skillManagerModel)
      && /@State private var selectedWorkflow:\s*SkillManagerWorkflow = \.searchInstall/.test(files.skillManager)
      && /Picker\(selection:\s*\$selectedWorkflow\)[\s\S]*?Label\(workflow\.title,\s*systemImage:\s*workflow\.systemImage\)\.tag\(workflow\)[\s\S]*?\.labelsHidden\(\)[\s\S]*?\.accessibilityLabel\(UIStrings\.text\("skillManager\.workflow\.accessibility"/.test(files.skillManager)
      && !/Picker\(UIStrings\.text\("skillManager\.workflow\.label"/.test(files.skillManager)
      && /private var workflowInputContent:[\s\S]*?case \.searchInstall:[\s\S]*?searchAndInstallControls[\s\S]*?case \.installedUpdates:[\s\S]*?installedActionControls[\s\S]*?case \.localLibrary:[\s\S]*?localLibraryControls/.test(files.skillManager)
      && /private var workflowResultsContent:[\s\S]*?case \.searchInstall:[\s\S]*?searchResultsSection[\s\S]*?case \.installedUpdates:[\s\S]*?installedResultsSection[\s\S]*?case \.localLibrary:[\s\S]*?localLibraryResultsSection/.test(files.skillManager)
      && /private var skillManagerFeedback:[\s\S]*?WorkflowSheetInlineBanner\(message:\s*error,\s*style:\s*\.error\)[\s\S]*?WorkflowSheetInlineBanner\(message:\s*message,\s*style:\s*\.success\)/.test(files.skillManager)
      && /externalMutationDisabled/.test(files.skillManager)
      && /externalManagerUnavailableMessage/.test(files.skillManager)
      && /toolUnavailableCard/.test(files.skillManager)
      && /search\.isBlockedByNetwork/.test(files.skillManager)
      && /skillManager\.search\.networkBlocked/.test(files.skillManager + "\n" + files.localizable)
      && /preview\.localizedSummary/.test(files.skillManager)
      && /var localizedSummary:\s*String/.test(files.skillManagerModel)
      && /store\.skillManagerErrorMessage/.test(files.skillManager)
      && /store\.skillManagerMessage/.test(files.skillManager)
      && /skillManagerInstallSkillName/.test(files.store + "\n" + files.skillManager)
      && /skillManagerRemoveSkillName/.test(files.store + "\n" + files.skillManager)
      && /clearSkillManagerWorkflowPreviews/.test(files.store + "\n" + files.skillManager + "\n" + files.sidebar)
      && /store\.skillManagerMutationConfirmation/.test(files.skillManager)
      && /confirmation\.inputs\.agents\.map\(DisplayText\.agent\)/.test(files.skillManager)
      && /\.accessibilityValue\(previewMatchAccessibilityValue\(matchesCurrentInputs\)\)/.test(files.skillManager)
      && /await store\.applySkillManagerInstall\(confirmation:\s*confirmation\)/.test(files.skillManager)
      && /await store\.applySkillManagerRemove\(confirmation:\s*confirmation\)/.test(files.skillManager)
      && /await store\.applySkillManagerUpdate\(confirmation:\s*confirmation\)/.test(files.skillManager)
      && /await store\.applySkillManagerLocalCreate\(confirmation:\s*confirmation\)/.test(files.skillManager)
      && /await store\.applySkillManagerLocalDelete\(confirmation:\s*confirmation\)/.test(files.skillManager)
      && !/private func applyCurrentMutation/.test(files.skillManager)
      && !/skillManagerMutationPreview/.test(files.store + "\n" + files.skillManager)
      && /private var canSearchSkillManager:[\s\S]*?skillManagerSearchQuery\.trimmingCharacters\(in:\s*\.whitespacesAndNewlines\)[\s\S]*?!store\.isSearchingSkillManager[\s\S]*?!externalMutationDisabled/.test(files.skillManager)
      && /\.disabled\(!canSearchSkillManager\)/.test(files.skillManager)
      && !/TextField\([\s\S]*?\$store\.skillManagerSkillName/.test(files.skillManager),
  },
  {
    label: "Skill Manager empty and blocked searches keep non-actionable completeness visible",
    passed: /search\.isBlockedByNetwork[\s\S]*?skillManager\.search\.networkBlocked[\s\S]*?else if search\.results\.isEmpty[\s\S]*?skillManager\.search\.noResults/.test(files.skillManager)
      && /\n {16}if let status = store\.skillManagerSearchStatus \{[\s\S]*?skillManagerSearchFooter\(status,\s*sourceCompleteness:\s*search\.sourceCompleteness\)/.test(files.skillManager)
      && /private func skillManagerSearchFooter[\s\S]*?if status\.canLoadMore[\s\S]*?loadMoreSkillManagerSearchResults[\s\S]*?showAllReturnedSkillManagerSearchResults/.test(files.skillManager)
      && /private func skillManagerSearchFooter[\s\S]*?status\.incompleteReason[\s\S]*?UIStrings\.listIncompleteReason[\s\S]*?skillManager\.search\.completenessRecovery/.test(files.skillManager)
      && /"skillManager\.search\.completenessRecovery"/.test(files.localizable)
      && /"skillManager\.search\.completenessRecovery"/.test(files.localizableZh),
  },
  {
    label: "Skill Manager target controls preserve compact selection and expose all agents",
    passed: /@State private var isShowingSkillManagerTargets = false/.test(files.skillManager)
      && /private var selectedTargetAgents:\s*\[SkillManagerAgent\][\s\S]*?SkillManagerAgent\.defaultTargets\.filter[\s\S]*?store\.skillManagerSelectedAgentIDs\.contains/.test(files.skillManager)
      && /private var targetControls:[\s\S]*?SkillManagerTargetSummary\(agents:\s*selectedTargetAgents\)[\s\S]*?Button\s*\{[\s\S]*?isShowingSkillManagerTargets\.toggle\(\)[\s\S]*?\} label:[\s\S]*?isShowingSkillManagerTargets \? "chevron\.up" : "chevron\.down"[\s\S]*?if isShowingSkillManagerTargets \{[\s\S]*?LazyVGrid/.test(files.skillManager)
      && /private struct SkillManagerTargetSummary:[\s\S]*?ExpandableSummaryList\([\s\S]*?agents,[\s\S]*?visibleLimit:\s*4,[\s\S]*?skill-manager-agents\.show-all[\s\S]*?SkillManagerTargetIcon\(agent:\s*agent\)[\s\S]*?UIStrings\.text\("skillManager\.agents\.allSelected"/.test(files.skillManager)
      && /private struct SkillManagerTargetIcon:[\s\S]*?AgentIconProvider\.image\(for:\s*agent\.skillAgentFilter\)[\s\S]*?Image\(nsImage:\s*image\)/.test(files.skillManager)
      && /private extension SkillManagerAgent\s*\{[\s\S]*?var skillAgentFilter:\s*SkillAgentFilter[\s\S]*?case \.hermesAgent:[\s\S]*?return \.hermes/.test(files.skillManager),
  },
  {
    label: "Skill Manager search suggestions render as clickable tag pills",
    passed: /private var skillManagerSearchSuggestions:\s*\[String\][\s\S]*?store\.localSkillLibrarySkills\.map\(\\\.name\)[\s\S]*?store\.skillManagerInstalled\?\.installed\.map\(\\\.name\)/.test(files.skillManager)
      && /skillManagerSuggestionBar/.test(files.skillManager)
      && /private var skillManagerSuggestionBar:[\s\S]*?ForEach\(skillManagerSearchSuggestions,\s*id:\s*\\\.self\)[\s\S]*?store\.skillManagerSearchQuery = suggestion[\s\S]*?SkillManagerSuggestionPill\(title:\s*suggestion\)[\s\S]*?\.help\(suggestion\)/.test(files.skillManager)
      && /private struct SkillManagerSuggestionPill:[\s\S]*?Text\(title\)[\s\S]*?\.background\(Color\.accentColor\.opacity\(0\.12\),\s*in:\s*Capsule\(\)\)/.test(files.skillManager),
  },
  {
    label: "retired smart analysis detail copy is removed from current UI resources",
    passed: !/Use focused smart analysis panels for quality scoring, task fit, and routing\./.test(files.detailSection)
      && !/"detail\.section\.analysis\.summary"/.test(files.localizable)
      && !/"detail\.analysisReview"/.test(files.localizable),
  },
  {
    label: "safe batch lives behind the skill-list batch operation sheet",
    passed: !/SafeBatchTogglePanel|BatchTogglePreviewSummary/.test(files.sidebar)
      && /SkillListSectionHeader\([\s\S]*?store\.resetBatchToggleSelectionToVisibleSkills\(\)[\s\S]*?isBatchOperationPresented\s*=\s*true/.test(files.sidebar)
      && /BatchSkillOperationSheet\(\)/.test(files.sidebar)
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
