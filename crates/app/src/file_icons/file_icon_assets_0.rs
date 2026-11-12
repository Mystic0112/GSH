pub(super) fn get(name: &str) -> Option<&'static [u8]> {
    match name {
        "3d" => Some(include_bytes!("../../assets/file_icons/3d.svg")),
        "abap" => Some(include_bytes!("../../assets/file_icons/abap.svg")),
        "abc" => Some(include_bytes!("../../assets/file_icons/abc.svg")),
        "actionscript" => Some(include_bytes!("../../assets/file_icons/actionscript.svg")),
        "ada" => Some(include_bytes!("../../assets/file_icons/ada.svg")),
        "adobe-illustrator" => Some(include_bytes!(
            "../../assets/file_icons/adobe-illustrator.svg"
        )),
        "adobe-photoshop" => Some(include_bytes!(
            "../../assets/file_icons/adobe-photoshop.svg"
        )),
        "adobe-swc" => Some(include_bytes!("../../assets/file_icons/adobe-swc.svg")),
        "adonis" => Some(include_bytes!("../../assets/file_icons/adonis.svg")),
        "advpl" => Some(include_bytes!("../../assets/file_icons/advpl.svg")),
        "advpl-include" => Some(include_bytes!("../../assets/file_icons/advpl-include.svg")),
        "advpl-ptm" => Some(include_bytes!("../../assets/file_icons/advpl-ptm.svg")),
        "advpl-tlpp" => Some(include_bytes!("../../assets/file_icons/advpl-tlpp.svg")),
        "amplify" => Some(include_bytes!("../../assets/file_icons/amplify.svg")),
        "android" => Some(include_bytes!("../../assets/file_icons/android.svg")),
        "angular" => Some(include_bytes!("../../assets/file_icons/angular.svg")),
        "angular-component" => Some(include_bytes!(
            "../../assets/file_icons/angular-component.svg"
        )),
        "angular-directive" => Some(include_bytes!(
            "../../assets/file_icons/angular-directive.svg"
        )),
        "angular-guard" => Some(include_bytes!("../../assets/file_icons/angular-guard.svg")),
        "angular-interceptor" => Some(include_bytes!(
            "../../assets/file_icons/angular-interceptor.svg"
        )),
        "angular-pipe" => Some(include_bytes!("../../assets/file_icons/angular-pipe.svg")),
        "angular-resolver" => Some(include_bytes!(
            "../../assets/file_icons/angular-resolver.svg"
        )),
        "angular-service" => Some(include_bytes!(
            "../../assets/file_icons/angular-service.svg"
        )),
        "antlr" => Some(include_bytes!("../../assets/file_icons/antlr.svg")),
        "apiblueprint" => Some(include_bytes!("../../assets/file_icons/apiblueprint.svg")),
        "apollo" => Some(include_bytes!("../../assets/file_icons/apollo.svg")),
        "applescript" => Some(include_bytes!("../../assets/file_icons/applescript.svg")),
        "apps-script" => Some(include_bytes!("../../assets/file_icons/apps-script.svg")),
        "appveyor" => Some(include_bytes!("../../assets/file_icons/appveyor.svg")),
        "appwrite" => Some(include_bytes!("../../assets/file_icons/appwrite.svg")),
        "architecture" => Some(include_bytes!("../../assets/file_icons/architecture.svg")),
        "arduino" => Some(include_bytes!("../../assets/file_icons/arduino.svg")),
        "asciidoc" => Some(include_bytes!("../../assets/file_icons/asciidoc.svg")),
        "assembly" => Some(include_bytes!("../../assets/file_icons/assembly.svg")),
        "astro" => Some(include_bytes!("../../assets/file_icons/astro.svg")),
        "astro-config" => Some(include_bytes!("../../assets/file_icons/astro-config.svg")),
        "astyle" => Some(include_bytes!("../../assets/file_icons/astyle.svg")),
        "audio" => Some(include_bytes!("../../assets/file_icons/audio.svg")),
        "aurelia" => Some(include_bytes!("../../assets/file_icons/aurelia.svg")),
        "authors" => Some(include_bytes!("../../assets/file_icons/authors.svg")),
        "auto" => Some(include_bytes!("../../assets/file_icons/auto.svg")),
        "autohotkey" => Some(include_bytes!("../../assets/file_icons/autohotkey.svg")),
        "autoit" => Some(include_bytes!("../../assets/file_icons/autoit.svg")),
        "azure" => Some(include_bytes!("../../assets/file_icons/azure.svg")),
        "azure-pipelines" => Some(include_bytes!(
            "../../assets/file_icons/azure-pipelines.svg"
        )),
        "babel" => Some(include_bytes!("../../assets/file_icons/babel.svg")),
        "ballerina" => Some(include_bytes!("../../assets/file_icons/ballerina.svg")),
        "bashly" => Some(include_bytes!("../../assets/file_icons/bashly.svg")),
        "bashly-settings" => Some(include_bytes!(
            "../../assets/file_icons/bashly-settings.svg"
        )),
        "bashly-strings" => Some(include_bytes!("../../assets/file_icons/bashly-strings.svg")),
        "bazel" => Some(include_bytes!("../../assets/file_icons/bazel.svg")),
        "bbx" => Some(include_bytes!("../../assets/file_icons/bbx.svg")),
        "beancount" => Some(include_bytes!("../../assets/file_icons/beancount.svg")),
        "bench-js" => Some(include_bytes!("../../assets/file_icons/bench-js.svg")),
        "bench-jsx" => Some(include_bytes!("../../assets/file_icons/bench-jsx.svg")),
        "bench-ts" => Some(include_bytes!("../../assets/file_icons/bench-ts.svg")),
        "bibliography" => Some(include_bytes!("../../assets/file_icons/bibliography.svg")),
        "bibtex-style" => Some(include_bytes!("../../assets/file_icons/bibtex-style.svg")),
        "bicep" => Some(include_bytes!("../../assets/file_icons/bicep.svg")),
        "biome" => Some(include_bytes!("../../assets/file_icons/biome.svg")),
        "bitbucket" => Some(include_bytes!("../../assets/file_icons/bitbucket.svg")),
        "bithound" => Some(include_bytes!("../../assets/file_icons/bithound.svg")),
        "blender" => Some(include_bytes!("../../assets/file_icons/blender.svg")),
        "blink" => Some(include_bytes!("../../assets/file_icons/blink.svg")),
        "blitz" => Some(include_bytes!("../../assets/file_icons/blitz.svg")),
        "bower" => Some(include_bytes!("../../assets/file_icons/bower.svg")),
        "brainfuck" => Some(include_bytes!("../../assets/file_icons/brainfuck.svg")),
        "browserlist" => Some(include_bytes!("../../assets/file_icons/browserlist.svg")),
        "bruno" => Some(include_bytes!("../../assets/file_icons/bruno.svg")),
        "buck" => Some(include_bytes!("../../assets/file_icons/buck.svg")),
        "bucklescript" => Some(include_bytes!("../../assets/file_icons/bucklescript.svg")),
        "buildkite" => Some(include_bytes!("../../assets/file_icons/buildkite.svg")),
        "bun" => Some(include_bytes!("../../assets/file_icons/bun.svg")),
        "c" => Some(include_bytes!("../../assets/file_icons/c.svg")),
        "c3" => Some(include_bytes!("../../assets/file_icons/c3.svg")),
        "cabal" => Some(include_bytes!("../../assets/file_icons/cabal.svg")),
        "caddy" => Some(include_bytes!("../../assets/file_icons/caddy.svg")),
        "cadence" => Some(include_bytes!("../../assets/file_icons/cadence.svg")),
        "cairo" => Some(include_bytes!("../../assets/file_icons/cairo.svg")),
        "cake" => Some(include_bytes!("../../assets/file_icons/cake.svg")),
        "capacitor" => Some(include_bytes!("../../assets/file_icons/capacitor.svg")),
        "capnp" => Some(include_bytes!("../../assets/file_icons/capnp.svg")),
        "cbx" => Some(include_bytes!("../../assets/file_icons/cbx.svg")),
        "cds" => Some(include_bytes!("../../assets/file_icons/cds.svg")),
        "certificate" => Some(include_bytes!("../../assets/file_icons/certificate.svg")),
        "changelog" => Some(include_bytes!("../../assets/file_icons/changelog.svg")),
        "chess" => Some(include_bytes!("../../assets/file_icons/chess.svg")),
        "chromatic" => Some(include_bytes!("../../assets/file_icons/chromatic.svg")),
        "chrome" => Some(include_bytes!("../../assets/file_icons/chrome.svg")),
        "circleci" => Some(include_bytes!("../../assets/file_icons/circleci.svg")),
        "citation" => Some(include_bytes!("../../assets/file_icons/citation.svg")),
        "clangd" => Some(include_bytes!("../../assets/file_icons/clangd.svg")),
        "claude" => Some(include_bytes!("../../assets/file_icons/claude.svg")),
        "cline" => Some(include_bytes!("../../assets/file_icons/cline.svg")),
        "clojure" => Some(include_bytes!("../../assets/file_icons/clojure.svg")),
        "cloudfoundry" => Some(include_bytes!("../../assets/file_icons/cloudfoundry.svg")),
        "cmake" => Some(include_bytes!("../../assets/file_icons/cmake.svg")),
        "coala" => Some(include_bytes!("../../assets/file_icons/coala.svg")),
        "cobol" => Some(include_bytes!("../../assets/file_icons/cobol.svg")),
        "coconut" => Some(include_bytes!("../../assets/file_icons/coconut.svg")),
        "code-climate" => Some(include_bytes!("../../assets/file_icons/code-climate.svg")),
        "codecov" => Some(include_bytes!("../../assets/file_icons/codecov.svg")),
        "codeowners" => Some(include_bytes!("../../assets/file_icons/codeowners.svg")),
        "coderabbit-ai" => Some(include_bytes!("../../assets/file_icons/coderabbit-ai.svg")),
        "coffee" => Some(include_bytes!("../../assets/file_icons/coffee.svg")),
        "coldfusion" => Some(include_bytes!("../../assets/file_icons/coldfusion.svg")),
        "coloredpetrinets" => Some(include_bytes!(
            "../../assets/file_icons/coloredpetrinets.svg"
        )),
        "command" => Some(include_bytes!("../../assets/file_icons/command.svg")),
        "commitizen" => Some(include_bytes!("../../assets/file_icons/commitizen.svg")),
        "commitlint" => Some(include_bytes!("../../assets/file_icons/commitlint.svg")),
        "concourse" => Some(include_bytes!("../../assets/file_icons/concourse.svg")),
        "conduct" => Some(include_bytes!("../../assets/file_icons/conduct.svg")),
        "console" => Some(include_bytes!("../../assets/file_icons/console.svg")),
        "container" => Some(include_bytes!("../../assets/file_icons/container.svg")),
        "contentlayer" => Some(include_bytes!("../../assets/file_icons/contentlayer.svg")),
        "context" => Some(include_bytes!("../../assets/file_icons/context.svg")),
        "contributing" => Some(include_bytes!("../../assets/file_icons/contributing.svg")),
        "controller" => Some(include_bytes!("../../assets/file_icons/controller.svg")),
        "copilot" => Some(include_bytes!("../../assets/file_icons/copilot.svg")),
        "cpp" => Some(include_bytes!("../../assets/file_icons/cpp.svg")),
        "craco" => Some(include_bytes!("../../assets/file_icons/craco.svg")),
        "credits" => Some(include_bytes!("../../assets/file_icons/credits.svg")),
        "crystal" => Some(include_bytes!("../../assets/file_icons/crystal.svg")),
        "csharp" => Some(include_bytes!("../../assets/file_icons/csharp.svg")),
        "css" => Some(include_bytes!("../../assets/file_icons/css.svg")),
        "css-map" => Some(include_bytes!("../../assets/file_icons/css-map.svg")),
        "cucumber" => Some(include_bytes!("../../assets/file_icons/cucumber.svg")),
        "cuda" => Some(include_bytes!("../../assets/file_icons/cuda.svg")),
        "cue" => Some(include_bytes!("../../assets/file_icons/cue.svg")),
        "cursor" => Some(include_bytes!("../../assets/file_icons/cursor.svg")),
        "cypress" => Some(include_bytes!("../../assets/file_icons/cypress.svg")),
        "d" => Some(include_bytes!("../../assets/file_icons/d.svg")),
        "dart" => Some(include_bytes!("../../assets/file_icons/dart.svg")),
        "dart_generated" => Some(include_bytes!("../../assets/file_icons/dart_generated.svg")),
        "database" => Some(include_bytes!("../../assets/file_icons/database.svg")),
        "deepsource" => Some(include_bytes!("../../assets/file_icons/deepsource.svg")),
        "denizenscript" => Some(include_bytes!("../../assets/file_icons/denizenscript.svg")),
        "deno" => Some(include_bytes!("../../assets/file_icons/deno.svg")),
        "dependabot" => Some(include_bytes!("../../assets/file_icons/dependabot.svg")),
        "dependencies-update" => Some(include_bytes!(
            "../../assets/file_icons/dependencies-update.svg"
        )),
        "dhall" => Some(include_bytes!("../../assets/file_icons/dhall.svg")),
        "diff" => Some(include_bytes!("../../assets/file_icons/diff.svg")),
        "dinophp" => Some(include_bytes!("../../assets/file_icons/dinophp.svg")),
        "disc" => Some(include_bytes!("../../assets/file_icons/disc.svg")),
        "django" => Some(include_bytes!("../../assets/file_icons/django.svg")),
        "dll" => Some(include_bytes!("../../assets/file_icons/dll.svg")),
        "docker" => Some(include_bytes!("../../assets/file_icons/docker.svg")),
        "doctex-installer" => Some(include_bytes!(
            "../../assets/file_icons/doctex-installer.svg"
        )),
        "document" => Some(include_bytes!("../../assets/file_icons/document.svg")),
        "dotjs" => Some(include_bytes!("../../assets/file_icons/dotjs.svg")),
        "drawio" => Some(include_bytes!("../../assets/file_icons/drawio.svg")),
        "drizzle" => Some(include_bytes!("../../assets/file_icons/drizzle.svg")),
        "drone" => Some(include_bytes!("../../assets/file_icons/drone.svg")),
        "dtx" => Some(include_bytes!("../../assets/file_icons/dtx.svg")),
        "duc" => Some(include_bytes!("../../assets/file_icons/duc.svg")),
        "dune" => Some(include_bytes!("../../assets/file_icons/dune.svg")),
        "edge" => Some(include_bytes!("../../assets/file_icons/edge.svg")),
        "editorconfig" => Some(include_bytes!("../../assets/file_icons/editorconfig.svg")),
        "ejs" => Some(include_bytes!("../../assets/file_icons/ejs.svg")),
        "elixir" => Some(include_bytes!("../../assets/file_icons/elixir.svg")),
        "elm" => Some(include_bytes!("../../assets/file_icons/elm.svg")),
        "email" => Some(include_bytes!("../../assets/file_icons/email.svg")),
        "ember" => Some(include_bytes!("../../assets/file_icons/ember.svg")),
        "epub" => Some(include_bytes!("../../assets/file_icons/epub.svg")),
        "erlang" => Some(include_bytes!("../../assets/file_icons/erlang.svg")),
        "esbuild" => Some(include_bytes!("../../assets/file_icons/esbuild.svg")),
        "eslint" => Some(include_bytes!("../../assets/file_icons/eslint.svg")),
        "excalidraw" => Some(include_bytes!("../../assets/file_icons/excalidraw.svg")),
        "exe" => Some(include_bytes!("../../assets/file_icons/exe.svg")),
        "expo" => Some(include_bytes!("../../assets/file_icons/expo.svg")),
        "fastlane" => Some(include_bytes!("../../assets/file_icons/fastlane.svg")),
        "favicon" => Some(include_bytes!("../../assets/file_icons/favicon.svg")),
        "figma" => Some(include_bytes!("../../assets/file_icons/figma.svg")),
        "file" => Some(include_bytes!("../../assets/file_icons/file.svg")),
        "firebase" => Some(include_bytes!("../../assets/file_icons/firebase.svg")),
        "flash" => Some(include_bytes!("../../assets/file_icons/flash.svg")),
        "flow" => Some(include_bytes!("../../assets/file_icons/flow.svg")),
        "folder" => Some(include_bytes!("../../assets/file_icons/folder.svg")),
        "folder-admin" => Some(include_bytes!("../../assets/file_icons/folder-admin.svg")),
        "folder-admin-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-admin-open.svg"
        )),
        "folder-android" => Some(include_bytes!("../../assets/file_icons/folder-android.svg")),
        "folder-android-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-android-open.svg"
        )),
        "folder-angular" => Some(include_bytes!("../../assets/file_icons/folder-angular.svg")),
        "folder-angular-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-angular-open.svg"
        )),
        "folder-animation" => Some(include_bytes!(
            "../../assets/file_icons/folder-animation.svg"
        )),
        "folder-animation-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-animation-open.svg"
        )),
        "folder-ansible" => Some(include_bytes!("../../assets/file_icons/folder-ansible.svg")),
        "folder-ansible-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-ansible-open.svg"
        )),
        "folder-api" => Some(include_bytes!("../../assets/file_icons/folder-api.svg")),
        "folder-api-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-api-open.svg"
        )),
        "folder-apollo" => Some(include_bytes!("../../assets/file_icons/folder-apollo.svg")),
        "folder-apollo-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-apollo-open.svg"
        )),
        "folder-app" => Some(include_bytes!("../../assets/file_icons/folder-app.svg")),
        "folder-app-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-app-open.svg"
        )),
        "folder-appwrite" => Some(include_bytes!(
            "../../assets/file_icons/folder-appwrite.svg"
        )),
        "folder-appwrite-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-appwrite-open.svg"
        )),
        "folder-archive" => Some(include_bytes!("../../assets/file_icons/folder-archive.svg")),
        "folder-archive-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-archive-open.svg"
        )),
        "folder-assembly" => Some(include_bytes!(
            "../../assets/file_icons/folder-assembly.svg"
        )),
        "folder-assembly-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-assembly-open.svg"
        )),
        "folder-astro" => Some(include_bytes!("../../assets/file_icons/folder-astro.svg")),
        "folder-astro-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-astro-open.svg"
        )),
        "folder-atom" => Some(include_bytes!("../../assets/file_icons/folder-atom.svg")),
        "folder-atom-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-atom-open.svg"
        )),
        "folder-attachment" => Some(include_bytes!(
            "../../assets/file_icons/folder-attachment.svg"
        )),
        "folder-attachment-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-attachment-open.svg"
        )),
        "folder-audio" => Some(include_bytes!("../../assets/file_icons/folder-audio.svg")),
        "folder-audio-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-audio-open.svg"
        )),
        "folder-aurelia" => Some(include_bytes!("../../assets/file_icons/folder-aurelia.svg")),
        "folder-aurelia-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-aurelia-open.svg"
        )),
        "folder-aws" => Some(include_bytes!("../../assets/file_icons/folder-aws.svg")),
        "folder-aws-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-aws-open.svg"
        )),
        "folder-azure-pipelines" => Some(include_bytes!(
            "../../assets/file_icons/folder-azure-pipelines.svg"
        )),
        "folder-azure-pipelines-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-azure-pipelines-open.svg"
        )),
        "folder-backup" => Some(include_bytes!("../../assets/file_icons/folder-backup.svg")),
        "folder-backup-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-backup-open.svg"
        )),
        "folder-base" => Some(include_bytes!("../../assets/file_icons/folder-base.svg")),
        "folder-base-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-base-open.svg"
        )),
        "folder-batch" => Some(include_bytes!("../../assets/file_icons/folder-batch.svg")),
        "folder-batch-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-batch-open.svg"
        )),
        "folder-benchmark" => Some(include_bytes!(
            "../../assets/file_icons/folder-benchmark.svg"
        )),
        "folder-benchmark-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-benchmark-open.svg"
        )),
        "folder-bibliography" => Some(include_bytes!(
            "../../assets/file_icons/folder-bibliography.svg"
        )),
        "folder-bibliography-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-bibliography-open.svg"
        )),
        "folder-bicep" => Some(include_bytes!("../../assets/file_icons/folder-bicep.svg")),
        "folder-bicep-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-bicep-open.svg"
        )),
        "folder-blender" => Some(include_bytes!("../../assets/file_icons/folder-blender.svg")),
        "folder-blender-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-blender-open.svg"
        )),
        "folder-bloc" => Some(include_bytes!("../../assets/file_icons/folder-bloc.svg")),
        "folder-bloc-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-bloc-open.svg"
        )),
        "folder-bower" => Some(include_bytes!("../../assets/file_icons/folder-bower.svg")),
        "folder-bower-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-bower-open.svg"
        )),
        "folder-buildkite" => Some(include_bytes!(
            "../../assets/file_icons/folder-buildkite.svg"
        )),
        "folder-buildkite-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-buildkite-open.svg"
        )),
        "folder-cart" => Some(include_bytes!("../../assets/file_icons/folder-cart.svg")),
        "folder-cart-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-cart-open.svg"
        )),
        "folder-changesets" => Some(include_bytes!(
            "../../assets/file_icons/folder-changesets.svg"
        )),
        "folder-changesets-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-changesets-open.svg"
        )),
        "folder-ci" => Some(include_bytes!("../../assets/file_icons/folder-ci.svg")),
        "folder-ci-open" => Some(include_bytes!("../../assets/file_icons/folder-ci-open.svg")),
        "folder-circleci" => Some(include_bytes!(
            "../../assets/file_icons/folder-circleci.svg"
        )),
        "folder-circleci-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-circleci-open.svg"
        )),
        "folder-class" => Some(include_bytes!("../../assets/file_icons/folder-class.svg")),
        "folder-class-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-class-open.svg"
        )),
        "folder-claude" => Some(include_bytes!("../../assets/file_icons/folder-claude.svg")),
        "folder-claude-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-claude-open.svg"
        )),
        "folder-client" => Some(include_bytes!("../../assets/file_icons/folder-client.svg")),
        "folder-client-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-client-open.svg"
        )),
        "folder-cline" => Some(include_bytes!("../../assets/file_icons/folder-cline.svg")),
        "folder-cline-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-cline-open.svg"
        )),
        "folder-cloud-functions" => Some(include_bytes!(
            "../../assets/file_icons/folder-cloud-functions.svg"
        )),
        "folder-cloud-functions-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-cloud-functions-open.svg"
        )),
        "folder-cloudflare" => Some(include_bytes!(
            "../../assets/file_icons/folder-cloudflare.svg"
        )),
        "folder-cloudflare-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-cloudflare-open.svg"
        )),
        "folder-cluster" => Some(include_bytes!("../../assets/file_icons/folder-cluster.svg")),
        "folder-cluster-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-cluster-open.svg"
        )),
        "folder-cobol" => Some(include_bytes!("../../assets/file_icons/folder-cobol.svg")),
        "folder-cobol-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-cobol-open.svg"
        )),
        "folder-command" => Some(include_bytes!("../../assets/file_icons/folder-command.svg")),
        "folder-command-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-command-open.svg"
        )),
        "folder-components" => Some(include_bytes!(
            "../../assets/file_icons/folder-components.svg"
        )),
        "folder-components-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-components-open.svg"
        )),
        "folder-config" => Some(include_bytes!("../../assets/file_icons/folder-config.svg")),
        "folder-config-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-config-open.svg"
        )),
        "folder-connection" => Some(include_bytes!(
            "../../assets/file_icons/folder-connection.svg"
        )),
        "folder-connection-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-connection-open.svg"
        )),
        "folder-console" => Some(include_bytes!("../../assets/file_icons/folder-console.svg")),
        "folder-console-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-console-open.svg"
        )),
        "folder-constant" => Some(include_bytes!(
            "../../assets/file_icons/folder-constant.svg"
        )),
        "folder-constant-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-constant-open.svg"
        )),
        "folder-container" => Some(include_bytes!(
            "../../assets/file_icons/folder-container.svg"
        )),
        "folder-container-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-container-open.svg"
        )),
        "folder-content" => Some(include_bytes!("../../assets/file_icons/folder-content.svg")),
        "folder-content-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-content-open.svg"
        )),
        "folder-context" => Some(include_bytes!("../../assets/file_icons/folder-context.svg")),
        "folder-context-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-context-open.svg"
        )),
        "folder-contract" => Some(include_bytes!(
            "../../assets/file_icons/folder-contract.svg"
        )),
        "folder-contract-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-contract-open.svg"
        )),
        "folder-controller" => Some(include_bytes!(
            "../../assets/file_icons/folder-controller.svg"
        )),
        "folder-controller-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-controller-open.svg"
        )),
        "folder-core" => Some(include_bytes!("../../assets/file_icons/folder-core.svg")),
        "folder-core-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-core-open.svg"
        )),
        "folder-coverage" => Some(include_bytes!(
            "../../assets/file_icons/folder-coverage.svg"
        )),
        "folder-coverage-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-coverage-open.svg"
        )),
        "folder-css" => Some(include_bytes!("../../assets/file_icons/folder-css.svg")),
        "folder-css-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-css-open.svg"
        )),
        "folder-cue" => Some(include_bytes!("../../assets/file_icons/folder-cue.svg")),
        "folder-cue-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-cue-open.svg"
        )),
        "folder-cursor" => Some(include_bytes!("../../assets/file_icons/folder-cursor.svg")),
        "folder-cursor-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-cursor-open.svg"
        )),
        "folder-custom" => Some(include_bytes!("../../assets/file_icons/folder-custom.svg")),
        "folder-custom-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-custom-open.svg"
        )),
        "folder-cypress" => Some(include_bytes!("../../assets/file_icons/folder-cypress.svg")),
        "folder-cypress-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-cypress-open.svg"
        )),
        "folder-dal" => Some(include_bytes!("../../assets/file_icons/folder-dal.svg")),
        "folder-dal-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-dal-open.svg"
        )),
        "folder-dart" => Some(include_bytes!("../../assets/file_icons/folder-dart.svg")),
        "folder-dart-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-dart-open.svg"
        )),
        "folder-database" => Some(include_bytes!(
            "../../assets/file_icons/folder-database.svg"
        )),
        "folder-database-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-database-open.svg"
        )),
        "folder-debug" => Some(include_bytes!("../../assets/file_icons/folder-debug.svg")),
        "folder-debug-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-debug-open.svg"
        )),
        "folder-decorators" => Some(include_bytes!(
            "../../assets/file_icons/folder-decorators.svg"
        )),
        "folder-decorators-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-decorators-open.svg"
        )),
        "folder-delta" => Some(include_bytes!("../../assets/file_icons/folder-delta.svg")),
        "folder-delta-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-delta-open.svg"
        )),
        "folder-deprecated" => Some(include_bytes!(
            "../../assets/file_icons/folder-deprecated.svg"
        )),
        "folder-deprecated-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-deprecated-open.svg"
        )),
        "folder-desktop" => Some(include_bytes!("../../assets/file_icons/folder-desktop.svg")),
        "folder-desktop-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-desktop-open.svg"
        )),
        "folder-development" => Some(include_bytes!(
            "../../assets/file_icons/folder-development.svg"
        )),
        "folder-development-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-development-open.svg"
        )),
        "folder-directive" => Some(include_bytes!(
            "../../assets/file_icons/folder-directive.svg"
        )),
        "folder-directive-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-directive-open.svg"
        )),
        "folder-dist" => Some(include_bytes!("../../assets/file_icons/folder-dist.svg")),
        "folder-dist-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-dist-open.svg"
        )),
        "folder-docker" => Some(include_bytes!("../../assets/file_icons/folder-docker.svg")),
        "folder-docker-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-docker-open.svg"
        )),
        "folder-docs" => Some(include_bytes!("../../assets/file_icons/folder-docs.svg")),
        "folder-docs-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-docs-open.svg"
        )),
        "folder-download" => Some(include_bytes!(
            "../../assets/file_icons/folder-download.svg"
        )),
        "folder-download-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-download-open.svg"
        )),
        "folder-drizzle" => Some(include_bytes!("../../assets/file_icons/folder-drizzle.svg")),
        "folder-drizzle-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-drizzle-open.svg"
        )),
        "folder-dump" => Some(include_bytes!("../../assets/file_icons/folder-dump.svg")),
        "folder-dump-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-dump-open.svg"
        )),
        "folder-eas" => Some(include_bytes!("../../assets/file_icons/folder-eas.svg")),
        "folder-eas-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-eas-open.svg"
        )),
        "folder-element" => Some(include_bytes!("../../assets/file_icons/folder-element.svg")),
        "folder-element-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-element-open.svg"
        )),
        "folder-enum" => Some(include_bytes!("../../assets/file_icons/folder-enum.svg")),
        "folder-enum-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-enum-open.svg"
        )),
        "folder-environment" => Some(include_bytes!(
            "../../assets/file_icons/folder-environment.svg"
        )),
        "folder-environment-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-environment-open.svg"
        )),
        "folder-error" => Some(include_bytes!("../../assets/file_icons/folder-error.svg")),
        "folder-error-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-error-open.svg"
        )),
        "folder-eslint" => Some(include_bytes!("../../assets/file_icons/folder-eslint.svg")),
        "folder-eslint-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-eslint-open.svg"
        )),
        "folder-event" => Some(include_bytes!("../../assets/file_icons/folder-event.svg")),
        "folder-event-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-event-open.svg"
        )),
        "folder-examples" => Some(include_bytes!(
            "../../assets/file_icons/folder-examples.svg"
        )),
        "folder-examples-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-examples-open.svg"
        )),
        "folder-expo" => Some(include_bytes!("../../assets/file_icons/folder-expo.svg")),
        "folder-expo-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-expo-open.svg"
        )),
        "folder-export" => Some(include_bytes!("../../assets/file_icons/folder-export.svg")),
        "folder-export-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-export-open.svg"
        )),
        "folder-fastlane" => Some(include_bytes!(
            "../../assets/file_icons/folder-fastlane.svg"
        )),
        "folder-fastlane-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-fastlane-open.svg"
        )),
        "folder-favicon" => Some(include_bytes!("../../assets/file_icons/folder-favicon.svg")),
        "folder-favicon-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-favicon-open.svg"
        )),
        "folder-features" => Some(include_bytes!(
            "../../assets/file_icons/folder-features.svg"
        )),
        "folder-features-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-features-open.svg"
        )),
        "folder-filter" => Some(include_bytes!("../../assets/file_icons/folder-filter.svg")),
        "folder-filter-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-filter-open.svg"
        )),
        "folder-firebase" => Some(include_bytes!(
            "../../assets/file_icons/folder-firebase.svg"
        )),
        "folder-firebase-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-firebase-open.svg"
        )),
        "folder-firestore" => Some(include_bytes!(
            "../../assets/file_icons/folder-firestore.svg"
        )),
        "folder-firestore-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-firestore-open.svg"
        )),
        "folder-flow" => Some(include_bytes!("../../assets/file_icons/folder-flow.svg")),
        "folder-flow-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-flow-open.svg"
        )),
        "folder-flutter" => Some(include_bytes!("../../assets/file_icons/folder-flutter.svg")),
        "folder-flutter-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-flutter-open.svg"
        )),
        "folder-font" => Some(include_bytes!("../../assets/file_icons/folder-font.svg")),
        "folder-font-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-font-open.svg"
        )),
        "folder-forgejo" => Some(include_bytes!("../../assets/file_icons/folder-forgejo.svg")),
        "folder-forgejo-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-forgejo-open.svg"
        )),
        "folder-form" => Some(include_bytes!("../../assets/file_icons/folder-form.svg")),
        "folder-form-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-form-open.svg"
        )),
        "folder-functions" => Some(include_bytes!(
            "../../assets/file_icons/folder-functions.svg"
        )),
        "folder-functions-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-functions-open.svg"
        )),
        "folder-gamemaker" => Some(include_bytes!(
            "../../assets/file_icons/folder-gamemaker.svg"
        )),
        "folder-gamemaker-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-gamemaker-open.svg"
        )),
        "folder-gemini-ai" => Some(include_bytes!(
            "../../assets/file_icons/folder-gemini-ai.svg"
        )),
        "folder-gemini-ai-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-gemini-ai-open.svg"
        )),
        "folder-generator" => Some(include_bytes!(
            "../../assets/file_icons/folder-generator.svg"
        )),
        _ => None,
    }
}
