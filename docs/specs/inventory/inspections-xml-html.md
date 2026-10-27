# Inspections: xml-html

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Accessibility (6)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `HtmlFormInputWithoutLabel` | Missing associated label | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `HtmlRequiredAltAttribute` | Missing required 'alt' attribute | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `HtmlRequiredLangAttribute` | Missing required 'lang' attribute | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `HtmlRequiredSummaryAttribute` | Missing required 'summary' attribute | XML | false | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `HtmlRequiredTitleAttribute` | Missing required 'title' attribute | XML | false | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `HtmlRequiredTitleElement` | Missing required 'title' element | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Ant (3)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `AntDuplicateTargetsInspection` | Duplicate targets | XML | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `AntMissingPropertiesFileInspection` | Missing properties file | XML | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `AntResolveInspection` | Ant references resolve problems | XML | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |

## HTML (16)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `CheckEmptyScriptTag` | Empty tag | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CheckImageSize` | Mismatched image size | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CheckValidXmlInScriptTagBody` | Malformed content of 'script' tag | XML | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `HtmlDeprecatedAttribute` | Obsolete attribute | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `HtmlDeprecatedTag` | Obsolete tag | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `HtmlExtraClosingTag` | Redundant closing tag | HTML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `HtmlMissingClosingTag` | Missing closing tag | HTML | true | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `HtmlNonExistentInternetResource` | Unresolved web link | XML | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `HtmlPresentationalElement` | Presentational tag | XML | false | INFORMATION | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `HtmlUnknownAnchorTarget` | Unresolved fragment in a link | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `HtmlUnknownAttribute` | Unknown attribute | HTML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `HtmlUnknownBooleanAttribute` | Incorrect boolean attribute | HTML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `HtmlUnknownTag` | Unknown tag | HTML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `HtmlUnknownTarget` | Unresolved file in a link | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `HtmlWrongAttributeValue` | Wrong attribute value | HtmlCompatible | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RequiredAttributes` | Missing required attribute | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Maven (16)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `Maven4RedundantParentCoordinates` | The parent coordinates are redundant and not required in Maven 4 | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MavenBomPackagingInOldSchema` | Model version 4.1.0 is required for the packaging | XML | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MavenDuplicateDependenciesInspection` | Duplicate Dependencies | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MavenDuplicatePluginInspection` | Duplicate plugin declaration | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MavenModelInspection` | Maven Model Inspection | XML | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MavenModelVersionMissed` | 'modelVersion' child tag should be defined | XML | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MavenModulesInMaven4` | The &lt;modules&gt; tag is deprecated; use &lt;subprojects&gt; instead | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MavenNewElementsInOldSchema` | Wrong model version. Model version should be 4.1.0 for this tag | XML | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MavenNewModelVersionInOldMaven` | Maven version 4+ is required for projects with 4.1.0 schema | XML | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MavenNewModelVersionInOldSchema` | Wrong model version. Model version should be 4.1.0 for this tag | XML | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MavenNoRootDefined` | Multi-module directory is not defined | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MavenParentMissedGroupIdArtifactIdInspection` | Parent groupId or artifactId is missing | XML | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MavenParentMissedVersionInspection` | Parent version missed | XML | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MavenPropertyInParent` | Usage of properties in parent description | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MavenRedundantGroupId` | Redundant groupId | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MavenRedundantVersion` | Redundant version | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## Plugin descriptor (28)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `AllContentModulesOptionalLoadingRule` | All content modules use optional loading rule | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `BackendActionInFrontendGroup` | Backend action registered in frontend action group | XML | false | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ContentModuleNamespace` | Content module namespace validation | XML | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ContentModuleVisibility` | Cannot access content module dependency | XML | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ContentModuleWithoutDedicatedJpsModule` | Content module without a dedicated IntelliJ IDEA module | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `EmptyPluginXmlTag` | Empty plugin XML container tag | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `JetBrainsInternalExtensionsUsage` | Usage of JetBrains internal extension point | XML | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `KeymapXmlDom` | Keymap XML validity | XML | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `KotlinObjectExtensionRegistration` | Extension class is a Kotlin object | XML | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `LightServiceMigrationXML` | A service can be converted to a light one | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `LiveTemplateI18n` | Live templates i18n | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `MissingFrontendOrBackendRuntimeDependency` | Missing frontend or backend runtime dependency | XML | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ModuleNotRegisteredAsPluginContent` | Module is not declared as content in any plugin | XML | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `OptionalDependencyViaDependsTag` | Optional dependency declared by 'depends' tag | XML | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PluginXmlCapitalization` | Plugin.xml text capitalization | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PluginXmlDynamicPlugin` | Plugin.xml dynamic plugin verification | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PluginXmlExtensionRegistration` | Plugin.xml extension registration | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PluginXmlI18n` | Plugin.xml i18n verification | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PluginXmlPluginLogo` | Plugin Logo check | XML | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PluginXmlReferencesModuleReachability` | Referenced element in plugin descriptor not reachable from module dependencies | XML | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PluginXmlRegistrationCheck` | Plugin.xml registration check | XML | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `PluginXmlValidity` | Plugin.xml validity | XML | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ProductModulesXmlDom` | Product-modules.xml validity | XML | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SplitModeImplicitModuleKind` | Implicit split mode module kind inspection | XML | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SplitModeMixedDependencies` | Split mode compatibility inspection | XML | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `SplitModeXmlApiUsage` | Inappropriate Frontend/Backend Extension registration in module | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnstableExtensionsUsage` | Usage of experimental extension point | XML | true | WEAK WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `WrongPluginModuleAlias` | Invalid plugin module alias | XML | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |

## RELAX NG (2)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `UnresolvedReference` | Unresolved reference |  | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `UnusedDefine` | Unused define |  | false | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## XML (13)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `CheckDtdRefs` | Unresolved DTD reference | XML | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CheckTagEmptyBody` | Empty element content | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `CheckXmlFileWithXercesValidator` | Failed external validation | XML | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `XmlDefaultAttributeValue` | Redundant attribute with default value | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `XmlDeprecatedElement` | Deprecated symbol | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `XmlDuplicatedId` | Duplicate 'id' attribute | XML | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `XmlHighlighting` | XML highlighting | XML | true | ERROR | global | framework: codeinsight/03-inspections.md (CI-INS) |
| `XmlInvalidId` | Unresolved 'id' reference | XML | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `XmlPathReference` | Unresolved file reference | XML | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `XmlUnboundNsPrefix` | Unbound namespace prefix | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `XmlUnresolvedReference` | Unresolved references | XML | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `XmlUnusedNamespaceDeclaration` | Unused schema declaration | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `XmlWrongRootElement` | Wrong root element | XML | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |

## XPath (5)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `CheckNodeTest` | Unknown element or attribute name | XPath | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `HardwiredNamespacePrefix` | Hardcoded namespace prefix | XPath | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `ImplicitTypeConversion` | Implicit type conversion | XPath | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `IndexZeroUsage` | XPath predicate with index 0 | XPath | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `RedundantTypeConversion` | Redundant type conversion | XPath | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |

## XSLT (4)

| Short name | Display name | Language | On by default | Level | Kind | Spec |
|---|---|---|---|---|---|---|
| `XsltDeclarations` | Incorrect declaration | XML | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `XsltTemplateInvocation` | Incorrect template invocation | XML | true | ERROR | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `XsltUnusedDeclaration` | Unused variable or parameter | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
| `XsltVariableShadowing` | Shadowed variable | XML | true | WARNING | local | framework: codeinsight/03-inspections.md (CI-INS) |
