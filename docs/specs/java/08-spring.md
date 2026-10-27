# 08. Spring (LATER)

Status: specification, clean-room. Written from what the IntelliJ IDEA **Community** sources contain, plus a
statement of what is not in them. Source paths are cited as `ic: <path>`.

## 8.1 Finding

Spring support in IntelliJ is part of the **Ultimate** edition (the Spring plugin, Spring Boot, Spring Data,
Spring Security, Spring MVC support, bean navigation, run configurations, `application.properties` code
insight, endpoints tool window, Spring Initializr in the New Project wizard). **None of that is in the
Community repository.** Evidence in `ic:`:

- There is no Spring plugin directory under `ic: plugins/` (the list is: Ant, Maven, Gradle, JUnit, TestNG,
  Lombok, JavaFX, Properties, YAML, TOML, Markdown, Groovy, Kotlin, ...). A search of the Java modules for
  `springframework` finds only the items in 8.2.
- The new-project wizard contains only a *promotional* entry for Spring (`ic: java/java-promo/`, plus
  promo icons under `ic: java/idea-ui/resources/icons/`) which tells the user the feature belongs to Ultimate.
- Library-usage statistics (`ic: java/java-impl/resources/com/intellij/internal/statistic/libraryJar/`) list
  Spring jars as "technologies" for telemetry only.

So "Java analysis exactly as good as IntelliJ's" for the Community edition means: **no Spring-specific
language intelligence beyond 8.2**. Spring-aware behavior that Ultimate users expect is a separate product
area and is specified below only as observable behavior to be designed later (8.3), not derived from source.

## 8.2 What Community does know about Spring

These are the only Spring touch points, all of them *library recognition by name* inside generic analyses:

| ID | Pri | Behavior | Where (ic) |
|---|---|---|---|
| JAVA-SPR-001 | SHOULD | The Spring `@Contract` annotation (`org.springframework.lang.Contract`) is accepted as a contract annotation with the same syntax as the JetBrains `@Contract` (the known-contract-annotation list holds exactly these two). **Not** in Community: Spring's `@Nullable`/`@NonNull`/`@NonNullApi` are *not* in the Community nullability annotation families (those are JetBrains, JSpecify, Checker Framework, Eclipse, SpotBugs/FindBugs, Jakarta, JSR-305 - see `07` 7.4.4); Spring and Android annotation support arrive through plugins. | `ic: java/java-psi-api/.../StaticAnalysisAnnotationManager.java`, `ic: java/java-impl/.../annoPackages/` |
| JAVA-SPR-002 | SHOULD | Hard-coded data-flow contracts: `org.springframework.util.Assert.isTrue/state(cond, ...)` fails when `cond` is false; `Assert.notNull(x, ...)` fails when `x` is null and returns its argument when it has a return value; `org.springframework.util.CollectionUtils.isEmpty(c)` returns true for `null` or an empty collection, else false. (Same rule family as Apache `Validate` and JUnit/TestNG/Hamcrest assertions.) These make `ConstantValue`/`DataFlowIssue` (07) quiet or correct after such calls. | `ic: java/java-analysis-impl/.../dataFlow/HardcodedContracts.java` |
| JAVA-SPR-003 | LATER | The `CheckReturnValue` annotation from `org.springframework.lang` is treated like other "result must be used" annotations by `IgnoreResultOfCall` (07 JAVA-INS-023). | `ic: java/java-impl-inspections/.../IgnoreResultOfCallInspection.java` |
| JAVA-SPR-004 | LATER | `AutoCloseableResource` (JAVA-INS-056) does not report values of type `org.springframework.context.ConfigurableApplicationContext` that are not closed in the same method (the context is usually managed elsewhere); several other well-known long-lived resource types are on the same ignore list. | `ic: java/java-impl-inspections/.../AutoCloseableResourceInspection.java` |
| JAVA-SPR-005 | LATER | The "equals does not check parameter class" inspection recognizes `org.springframework.data.util.ProxyUtils.getUserClass(x)` as a class check. | `ic: java/java-analysis-impl/.../EqualsWhichDoesntCheckParameterClassInspection.java` |

Acceptance for 8.2: a method calling `Assert.notNull(x, "msg"); x.foo();` with `x` possibly null before the
call has no `Method invocation may produce NullPointerException` warning on `foo`; the same without the
`Assert` call does.

Implementation note: these are table-driven ("known methods and annotations by qualified name"), so they
belong to the shared *library knowledge* tables of the nullability/data-flow engine, not to a Spring module.

## 8.3 What would be needed for Ultimate-level Spring support (observable behavior only)

Not derived from source (the sources are not in Community); listed from what users see in IntelliJ Ultimate
so that the Archipel roadmap can size it. Each item is `LATER` and out of the Phase 4 gate.

- **JAVA-SPR-010** Bean model: discovery of beans declared by `@Component` and stereotypes (`@Service`,
  `@Repository`, `@Controller`, `@RestController`), `@Configuration` + `@Bean` methods, `@Import`,
  `@ComponentScan` packages, `@ConfigurationProperties` classes, XML bean definitions; the bean name and type
  per declaration; qualifiers (`@Qualifier`, `@Primary`), profiles (`@Profile`), conditions
  (`@ConditionalOn...`) (*approximate*).
- **JAVA-SPR-011** Injection resolution: from `@Autowired`/`@Inject`/`@Resource`/constructor injection points,
  resolve to candidate beans (by type, then qualifier/name); error markers `Could not autowire. No beans of
  'X' type found.` / `Could not autowire. There is more than one bean of 'X' type.`; gutter icons both ways
  (injection point to bean and bean to injection points); `Go to` from a bean to its injection points.
- **JAVA-SPR-012** Configuration properties: completion, navigation and unresolved-key warnings in
  `application.properties`/`application.yml` using the metadata JSON that Spring Boot libraries ship
  (`META-INF/spring-configuration-metadata.json`, `additional-spring-configuration-metadata.json`), including
  `@Value("${key}")` placeholders and `@ConfigurationProperties(prefix=...)` field binding.
- **JAVA-SPR-013** Web: request-mapping endpoints (`@GetMapping` ... with path composition), Endpoints tool
  window, URL search in Search Everywhere, `@PathVariable` / `@RequestParam` consistency checks, HTTP client
  file generation (LATER, Nexus HTTP pack synergy).
- **JAVA-SPR-014** Data: Spring Data repository method name parsing (`findByNameAndAge`) with property
  completion and validation, JPQL/`@Query` injection (language injection of a JPQL/SQL fragment).
- **JAVA-SPR-015** Run: Spring Boot run configuration (main class detection, active profiles, JVM options,
  live reload), actuator endpoints display. (The generic Java run/debug pack comes first.)
- **JAVA-SPR-016** Initializr-style new project wizard.

Recommended sequencing: first the generic Java pack with the 8.2 recognition tables, then a plugin-style Spring
pack built on the Phase 1 extension points (stub-indexed annotation search by short name makes bean
discovery cheap: the *annotations* stub index of `01` 1.3 is the enabling mechanism, and the *super classes*
index for `@Component`-meta-annotated types is not needed because annotation meta-hierarchy is resolved by
following annotation declarations).

## 8.4 Open questions

- **Q-SPR-1** Whether Archipel's roadmap treats Spring as part of "IntelliJ/Java pack" or as a paid/optional
  pack like IntelliJ's edition split. The editions roadmap lists IntelliJ IDEA (Java, Kotlin, Groovy) without
  Spring; this file assumes the latter.
- **Q-SPR-2** Message texts in 8.3 are recalled from IntelliJ Ultimate's UI and have **not** been verified
  against source; treat them as indicative.
