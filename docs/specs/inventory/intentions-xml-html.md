# Intentions: xml-html

Facts extracted mechanically from plugin descriptors and message bundles (see `README.md`). Spec column: `<spec file> <requirement>` = named or described in an existing spec, `framework: ...` = only the generic framework is specified, `MISSING` = no spec.

## Intentions (20)

Description = first sentence of the shipped description resource, if any.

| Class | Language | Category | Description | Spec |
|---|---|---|---|---|
| `ChooseFileIntentionAction` | XML |  | Adds path to a .jar file declared as a system property in pom.xml . | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `MyUpdateIndicesIntention` | XML |  |  | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `HtmlUpdateImageSizeIntention` | XML | HTML | Inserts or updates the width and height attributes of an &lt;img&gt; tag according to the actual size of the image t… | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `AddSchemaPrefixIntention` | XML | XML | Inserts a namespace prefix. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `AddXsiSchemaLocationForExtResourceAction` | XML | XML | Adds an xsi:schemaLocation attribute for an external resource. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `CDataToTextIntention` | XML | XML | Unwraps a text fragment from the CDATA section and escapes characters if necessary. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `CollapseTagIntention` | XML | XML | Replaces an opening tag and the corresponding closing tag with an equivalent self-closing tag. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertSchemaPrefixToDefaultIntention` | XML | XML | Removes the selected namespace prefix. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `FetchExtResourceAction` | XML | XML | Downloads a resource with the specified URL from the internet. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ManuallySetupExtResourceAction` | XML | XML | Maps a namespace to an XML schema file (DTD or XSD). | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `RearrangeAttributesIntention` | XML | XML | Rearranges the attributes of a tag. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `RemoveAttributeIntentionFix` | XML | XML | Removes an attribute from a tag. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `RemoveTagAndPromoteChildrenIntentionAction` | XML | XML | Removes an enclosing tag and promotes its children. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `TextToCDataIntention` | XML | XML | Wraps a text fragment in a CDATA section and removes escaping from characters if necessary. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `XmlChooseColorIntentionAction` | XML | XML | Opens the color picker for the value of an attribute. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `XmlSplitTagAction` | XML | XML | Splits the tag at the caret. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `AddOptionalArgumentsIntention` | $XSLT | XSLT | Adds missing arguments to a template invocation ( xsl:call-template ). | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ConvertIfToChooseIntention` | $XSLT | XSLT | Transforms a simple xsl:if into an xsl:choose construct that allows multiple xsl:when branches and an xsl:othe… | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `InlineXslAttribute` | $XSLT | XSLT | Inlines an xsl:attribute instruction into an Attribute Value Template. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
| `ReplaceWithXslAttribute` | $XSLT | XSLT | Extracts an attribute of a literal result element and converts it into an xsl:attribute instruction. | framework: codeinsight/04-intentions-and-alt-enter.md (CI-FIX) |
