pub(super) fn get(name: &str) -> Option<&'static [u8]> {
    match name {
        "folder-generator-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-generator-open.svg"
        )),
        "folder-gh-workflows" => Some(include_bytes!(
            "../../assets/file_icons/folder-gh-workflows.svg"
        )),
        "folder-gh-workflows-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-gh-workflows-open.svg"
        )),
        "folder-git" => Some(include_bytes!("../../assets/file_icons/folder-git.svg")),
        "folder-git-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-git-open.svg"
        )),
        "folder-gitea" => Some(include_bytes!("../../assets/file_icons/folder-gitea.svg")),
        "folder-gitea-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-gitea-open.svg"
        )),
        "folder-gitea-workflows" => Some(include_bytes!(
            "../../assets/file_icons/folder-gitea-workflows.svg"
        )),
        "folder-gitea-workflows-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-gitea-workflows-open.svg"
        )),
        "folder-github" => Some(include_bytes!("../../assets/file_icons/folder-github.svg")),
        "folder-github-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-github-open.svg"
        )),
        "folder-gitlab" => Some(include_bytes!("../../assets/file_icons/folder-gitlab.svg")),
        "folder-gitlab-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-gitlab-open.svg"
        )),
        "folder-global" => Some(include_bytes!("../../assets/file_icons/folder-global.svg")),
        "folder-global-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-global-open.svg"
        )),
        "folder-go" => Some(include_bytes!("../../assets/file_icons/folder-go.svg")),
        "folder-go-open" => Some(include_bytes!("../../assets/file_icons/folder-go-open.svg")),
        "folder-godot" => Some(include_bytes!("../../assets/file_icons/folder-godot.svg")),
        "folder-godot-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-godot-open.svg"
        )),
        "folder-gradle" => Some(include_bytes!("../../assets/file_icons/folder-gradle.svg")),
        "folder-gradle-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-gradle-open.svg"
        )),
        "folder-graphql" => Some(include_bytes!("../../assets/file_icons/folder-graphql.svg")),
        "folder-graphql-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-graphql-open.svg"
        )),
        "folder-guard" => Some(include_bytes!("../../assets/file_icons/folder-guard.svg")),
        "folder-guard-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-guard-open.svg"
        )),
        "folder-gulp" => Some(include_bytes!("../../assets/file_icons/folder-gulp.svg")),
        "folder-gulp-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-gulp-open.svg"
        )),
        "folder-helm" => Some(include_bytes!("../../assets/file_icons/folder-helm.svg")),
        "folder-helm-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-helm-open.svg"
        )),
        "folder-helper" => Some(include_bytes!("../../assets/file_icons/folder-helper.svg")),
        "folder-helper-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-helper-open.svg"
        )),
        "folder-home" => Some(include_bytes!("../../assets/file_icons/folder-home.svg")),
        "folder-home-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-home-open.svg"
        )),
        "folder-hook" => Some(include_bytes!("../../assets/file_icons/folder-hook.svg")),
        "folder-hook-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-hook-open.svg"
        )),
        "folder-husky" => Some(include_bytes!("../../assets/file_icons/folder-husky.svg")),
        "folder-husky-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-husky-open.svg"
        )),
        "folder-i18n" => Some(include_bytes!("../../assets/file_icons/folder-i18n.svg")),
        "folder-i18n-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-i18n-open.svg"
        )),
        "folder-images" => Some(include_bytes!("../../assets/file_icons/folder-images.svg")),
        "folder-images-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-images-open.svg"
        )),
        "folder-import" => Some(include_bytes!("../../assets/file_icons/folder-import.svg")),
        "folder-import-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-import-open.svg"
        )),
        "folder-include" => Some(include_bytes!("../../assets/file_icons/folder-include.svg")),
        "folder-include-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-include-open.svg"
        )),
        "folder-input" => Some(include_bytes!("../../assets/file_icons/folder-input.svg")),
        "folder-input-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-input-open.svg"
        )),
        "folder-instructions" => Some(include_bytes!(
            "../../assets/file_icons/folder-instructions.svg"
        )),
        "folder-instructions-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-instructions-open.svg"
        )),
        "folder-intellij" => Some(include_bytes!(
            "../../assets/file_icons/folder-intellij.svg"
        )),
        "folder-intellij-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-intellij-open.svg"
        )),
        "folder-interceptor" => Some(include_bytes!(
            "../../assets/file_icons/folder-interceptor.svg"
        )),
        "folder-interceptor-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-interceptor-open.svg"
        )),
        "folder-interface" => Some(include_bytes!(
            "../../assets/file_icons/folder-interface.svg"
        )),
        "folder-interface-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-interface-open.svg"
        )),
        "folder-ios" => Some(include_bytes!("../../assets/file_icons/folder-ios.svg")),
        "folder-ios-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-ios-open.svg"
        )),
        "folder-java" => Some(include_bytes!("../../assets/file_icons/folder-java.svg")),
        "folder-java-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-java-open.svg"
        )),
        "folder-javascript" => Some(include_bytes!(
            "../../assets/file_icons/folder-javascript.svg"
        )),
        "folder-javascript-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-javascript-open.svg"
        )),
        "folder-jinja" => Some(include_bytes!("../../assets/file_icons/folder-jinja.svg")),
        "folder-jinja-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-jinja-open.svg"
        )),
        "folder-job" => Some(include_bytes!("../../assets/file_icons/folder-job.svg")),
        "folder-job-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-job-open.svg"
        )),
        "folder-json" => Some(include_bytes!("../../assets/file_icons/folder-json.svg")),
        "folder-json-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-json-open.svg"
        )),
        "folder-jupyter" => Some(include_bytes!("../../assets/file_icons/folder-jupyter.svg")),
        "folder-jupyter-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-jupyter-open.svg"
        )),
        "folder-keys" => Some(include_bytes!("../../assets/file_icons/folder-keys.svg")),
        "folder-keys-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-keys-open.svg"
        )),
        "folder-kotlin" => Some(include_bytes!("../../assets/file_icons/folder-kotlin.svg")),
        "folder-kotlin-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-kotlin-open.svg"
        )),
        "folder-kubernetes" => Some(include_bytes!(
            "../../assets/file_icons/folder-kubernetes.svg"
        )),
        "folder-kubernetes-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-kubernetes-open.svg"
        )),
        "folder-kusto" => Some(include_bytes!("../../assets/file_icons/folder-kusto.svg")),
        "folder-kusto-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-kusto-open.svg"
        )),
        "folder-layout" => Some(include_bytes!("../../assets/file_icons/folder-layout.svg")),
        "folder-layout-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-layout-open.svg"
        )),
        "folder-lefthook" => Some(include_bytes!(
            "../../assets/file_icons/folder-lefthook.svg"
        )),
        "folder-lefthook-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-lefthook-open.svg"
        )),
        "folder-less" => Some(include_bytes!("../../assets/file_icons/folder-less.svg")),
        "folder-less-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-less-open.svg"
        )),
        "folder-lib" => Some(include_bytes!("../../assets/file_icons/folder-lib.svg")),
        "folder-lib-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-lib-open.svg"
        )),
        "folder-license" => Some(include_bytes!("../../assets/file_icons/folder-license.svg")),
        "folder-license-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-license-open.svg"
        )),
        "folder-link" => Some(include_bytes!("../../assets/file_icons/folder-link.svg")),
        "folder-link-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-link-open.svg"
        )),
        "folder-linux" => Some(include_bytes!("../../assets/file_icons/folder-linux.svg")),
        "folder-linux-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-linux-open.svg"
        )),
        "folder-liquibase" => Some(include_bytes!(
            "../../assets/file_icons/folder-liquibase.svg"
        )),
        "folder-liquibase-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-liquibase-open.svg"
        )),
        "folder-log" => Some(include_bytes!("../../assets/file_icons/folder-log.svg")),
        "folder-log-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-log-open.svg"
        )),
        "folder-lottie" => Some(include_bytes!("../../assets/file_icons/folder-lottie.svg")),
        "folder-lottie-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-lottie-open.svg"
        )),
        "folder-lua" => Some(include_bytes!("../../assets/file_icons/folder-lua.svg")),
        "folder-lua-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-lua-open.svg"
        )),
        "folder-luau" => Some(include_bytes!("../../assets/file_icons/folder-luau.svg")),
        "folder-luau-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-luau-open.svg"
        )),
        "folder-macos" => Some(include_bytes!("../../assets/file_icons/folder-macos.svg")),
        "folder-macos-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-macos-open.svg"
        )),
        "folder-mail" => Some(include_bytes!("../../assets/file_icons/folder-mail.svg")),
        "folder-mail-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-mail-open.svg"
        )),
        "folder-mappings" => Some(include_bytes!(
            "../../assets/file_icons/folder-mappings.svg"
        )),
        "folder-mappings-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-mappings-open.svg"
        )),
        "folder-markdown" => Some(include_bytes!(
            "../../assets/file_icons/folder-markdown.svg"
        )),
        "folder-markdown-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-markdown-open.svg"
        )),
        "folder-mercurial" => Some(include_bytes!(
            "../../assets/file_icons/folder-mercurial.svg"
        )),
        "folder-mercurial-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-mercurial-open.svg"
        )),
        "folder-messages" => Some(include_bytes!(
            "../../assets/file_icons/folder-messages.svg"
        )),
        "folder-messages-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-messages-open.svg"
        )),
        "folder-meta" => Some(include_bytes!("../../assets/file_icons/folder-meta.svg")),
        "folder-meta-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-meta-open.svg"
        )),
        "folder-metro" => Some(include_bytes!("../../assets/file_icons/folder-metro.svg")),
        "folder-metro-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-metro-open.svg"
        )),
        "folder-middleware" => Some(include_bytes!(
            "../../assets/file_icons/folder-middleware.svg"
        )),
        "folder-middleware-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-middleware-open.svg"
        )),
        "folder-migrations" => Some(include_bytes!(
            "../../assets/file_icons/folder-migrations.svg"
        )),
        "folder-migrations-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-migrations-open.svg"
        )),
        "folder-mjml" => Some(include_bytes!("../../assets/file_icons/folder-mjml.svg")),
        "folder-mjml-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-mjml-open.svg"
        )),
        "folder-mobile" => Some(include_bytes!("../../assets/file_icons/folder-mobile.svg")),
        "folder-mobile-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-mobile-open.svg"
        )),
        "folder-mock" => Some(include_bytes!("../../assets/file_icons/folder-mock.svg")),
        "folder-mock-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-mock-open.svg"
        )),
        "folder-mojo" => Some(include_bytes!("../../assets/file_icons/folder-mojo.svg")),
        "folder-mojo-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-mojo-open.svg"
        )),
        "folder-molecule" => Some(include_bytes!(
            "../../assets/file_icons/folder-molecule.svg"
        )),
        "folder-molecule-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-molecule-open.svg"
        )),
        "folder-moon" => Some(include_bytes!("../../assets/file_icons/folder-moon.svg")),
        "folder-moon-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-moon-open.svg"
        )),
        "folder-netlify" => Some(include_bytes!("../../assets/file_icons/folder-netlify.svg")),
        "folder-netlify-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-netlify-open.svg"
        )),
        "folder-next" => Some(include_bytes!("../../assets/file_icons/folder-next.svg")),
        "folder-next-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-next-open.svg"
        )),
        "folder-nginx" => Some(include_bytes!("../../assets/file_icons/folder-nginx.svg")),
        "folder-nginx-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-nginx-open.svg"
        )),
        "folder-node" => Some(include_bytes!("../../assets/file_icons/folder-node.svg")),
        "folder-node-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-node-open.svg"
        )),
        "folder-nuxt" => Some(include_bytes!("../../assets/file_icons/folder-nuxt.svg")),
        "folder-nuxt-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-nuxt-open.svg"
        )),
        "folder-obsidian" => Some(include_bytes!(
            "../../assets/file_icons/folder-obsidian.svg"
        )),
        "folder-obsidian-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-obsidian-open.svg"
        )),
        "folder-open" => Some(include_bytes!("../../assets/file_icons/folder-open.svg")),
        "folder-organism" => Some(include_bytes!(
            "../../assets/file_icons/folder-organism.svg"
        )),
        "folder-organism-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-organism-open.svg"
        )),
        "folder-other" => Some(include_bytes!("../../assets/file_icons/folder-other.svg")),
        "folder-other-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-other-open.svg"
        )),
        "folder-packages" => Some(include_bytes!(
            "../../assets/file_icons/folder-packages.svg"
        )),
        "folder-packages-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-packages-open.svg"
        )),
        "folder-pdf" => Some(include_bytes!("../../assets/file_icons/folder-pdf.svg")),
        "folder-pdf-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-pdf-open.svg"
        )),
        "folder-pdm" => Some(include_bytes!("../../assets/file_icons/folder-pdm.svg")),
        "folder-pdm-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-pdm-open.svg"
        )),
        "folder-php" => Some(include_bytes!("../../assets/file_icons/folder-php.svg")),
        "folder-php-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-php-open.svg"
        )),
        "folder-phpmailer" => Some(include_bytes!(
            "../../assets/file_icons/folder-phpmailer.svg"
        )),
        "folder-phpmailer-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-phpmailer-open.svg"
        )),
        "folder-pipe" => Some(include_bytes!("../../assets/file_icons/folder-pipe.svg")),
        "folder-pipe-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-pipe-open.svg"
        )),
        "folder-plastic" => Some(include_bytes!("../../assets/file_icons/folder-plastic.svg")),
        "folder-plastic-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-plastic-open.svg"
        )),
        "folder-plugin" => Some(include_bytes!("../../assets/file_icons/folder-plugin.svg")),
        "folder-plugin-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-plugin-open.svg"
        )),
        "folder-policy" => Some(include_bytes!("../../assets/file_icons/folder-policy.svg")),
        "folder-policy-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-policy-open.svg"
        )),
        "folder-postman" => Some(include_bytes!("../../assets/file_icons/folder-postman.svg")),
        "folder-postman-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-postman-open.svg"
        )),
        "folder-powershell" => Some(include_bytes!(
            "../../assets/file_icons/folder-powershell.svg"
        )),
        "folder-powershell-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-powershell-open.svg"
        )),
        "folder-prisma" => Some(include_bytes!("../../assets/file_icons/folder-prisma.svg")),
        "folder-prisma-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-prisma-open.svg"
        )),
        "folder-private" => Some(include_bytes!("../../assets/file_icons/folder-private.svg")),
        "folder-private-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-private-open.svg"
        )),
        "folder-project" => Some(include_bytes!("../../assets/file_icons/folder-project.svg")),
        "folder-project-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-project-open.svg"
        )),
        "folder-prompts" => Some(include_bytes!("../../assets/file_icons/folder-prompts.svg")),
        "folder-prompts-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-prompts-open.svg"
        )),
        "folder-proto" => Some(include_bytes!("../../assets/file_icons/folder-proto.svg")),
        "folder-proto-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-proto-open.svg"
        )),
        "folder-public" => Some(include_bytes!("../../assets/file_icons/folder-public.svg")),
        "folder-public-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-public-open.svg"
        )),
        "folder-python" => Some(include_bytes!("../../assets/file_icons/folder-python.svg")),
        "folder-python-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-python-open.svg"
        )),
        "folder-pytorch" => Some(include_bytes!("../../assets/file_icons/folder-pytorch.svg")),
        "folder-pytorch-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-pytorch-open.svg"
        )),
        "folder-quasar" => Some(include_bytes!("../../assets/file_icons/folder-quasar.svg")),
        "folder-quasar-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-quasar-open.svg"
        )),
        "folder-queue" => Some(include_bytes!("../../assets/file_icons/folder-queue.svg")),
        "folder-queue-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-queue-open.svg"
        )),
        "folder-r" => Some(include_bytes!("../../assets/file_icons/folder-r.svg")),
        "folder-r-open" => Some(include_bytes!("../../assets/file_icons/folder-r-open.svg")),
        "folder-redis" => Some(include_bytes!("../../assets/file_icons/folder-redis.svg")),
        "folder-redis-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-redis-open.svg"
        )),
        "folder-repository" => Some(include_bytes!(
            "../../assets/file_icons/folder-repository.svg"
        )),
        "folder-repository-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-repository-open.svg"
        )),
        "folder-resolver" => Some(include_bytes!(
            "../../assets/file_icons/folder-resolver.svg"
        )),
        "folder-resolver-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-resolver-open.svg"
        )),
        "folder-resource" => Some(include_bytes!(
            "../../assets/file_icons/folder-resource.svg"
        )),
        "folder-resource-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-resource-open.svg"
        )),
        "folder-review" => Some(include_bytes!("../../assets/file_icons/folder-review.svg")),
        "folder-review-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-review-open.svg"
        )),
        "folder-robot" => Some(include_bytes!("../../assets/file_icons/folder-robot.svg")),
        "folder-robot-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-robot-open.svg"
        )),
        "folder-routes" => Some(include_bytes!("../../assets/file_icons/folder-routes.svg")),
        "folder-routes-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-routes-open.svg"
        )),
        "folder-rules" => Some(include_bytes!("../../assets/file_icons/folder-rules.svg")),
        "folder-rules-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-rules-open.svg"
        )),
        "folder-rust" => Some(include_bytes!("../../assets/file_icons/folder-rust.svg")),
        "folder-rust-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-rust-open.svg"
        )),
        "folder-salt" => Some(include_bytes!("../../assets/file_icons/folder-salt.svg")),
        "folder-salt-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-salt-open.svg"
        )),
        "folder-sandbox" => Some(include_bytes!("../../assets/file_icons/folder-sandbox.svg")),
        "folder-sandbox-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-sandbox-open.svg"
        )),
        "folder-sass" => Some(include_bytes!("../../assets/file_icons/folder-sass.svg")),
        "folder-sass-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-sass-open.svg"
        )),
        "folder-scala" => Some(include_bytes!("../../assets/file_icons/folder-scala.svg")),
        "folder-scala-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-scala-open.svg"
        )),
        "folder-scons" => Some(include_bytes!("../../assets/file_icons/folder-scons.svg")),
        "folder-scons-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-scons-open.svg"
        )),
        "folder-scrap" => Some(include_bytes!("../../assets/file_icons/folder-scrap.svg")),
        "folder-scrap-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-scrap-open.svg"
        )),
        "folder-scripts" => Some(include_bytes!("../../assets/file_icons/folder-scripts.svg")),
        "folder-scripts-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-scripts-open.svg"
        )),
        "folder-secure" => Some(include_bytes!("../../assets/file_icons/folder-secure.svg")),
        "folder-secure-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-secure-open.svg"
        )),
        "folder-seeders" => Some(include_bytes!("../../assets/file_icons/folder-seeders.svg")),
        "folder-seeders-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-seeders-open.svg"
        )),
        "folder-server" => Some(include_bytes!("../../assets/file_icons/folder-server.svg")),
        "folder-server-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-server-open.svg"
        )),
        "folder-serverless" => Some(include_bytes!(
            "../../assets/file_icons/folder-serverless.svg"
        )),
        "folder-serverless-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-serverless-open.svg"
        )),
        "folder-shader" => Some(include_bytes!("../../assets/file_icons/folder-shader.svg")),
        "folder-shader-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-shader-open.svg"
        )),
        "folder-shared" => Some(include_bytes!("../../assets/file_icons/folder-shared.svg")),
        "folder-shared-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-shared-open.svg"
        )),
        "folder-simulations" => Some(include_bytes!(
            "../../assets/file_icons/folder-simulations.svg"
        )),
        "folder-simulations-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-simulations-open.svg"
        )),
        "folder-skills" => Some(include_bytes!("../../assets/file_icons/folder-skills.svg")),
        "folder-skills-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-skills-open.svg"
        )),
        "folder-snapcraft" => Some(include_bytes!(
            "../../assets/file_icons/folder-snapcraft.svg"
        )),
        "folder-snapcraft-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-snapcraft-open.svg"
        )),
        "folder-snippet" => Some(include_bytes!("../../assets/file_icons/folder-snippet.svg")),
        "folder-snippet-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-snippet-open.svg"
        )),
        "folder-src" => Some(include_bytes!("../../assets/file_icons/folder-src.svg")),
        "folder-src-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-src-open.svg"
        )),
        "folder-src-tauri" => Some(include_bytes!(
            "../../assets/file_icons/folder-src-tauri.svg"
        )),
        "folder-src-tauri-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-src-tauri-open.svg"
        )),
        "folder-stack" => Some(include_bytes!("../../assets/file_icons/folder-stack.svg")),
        "folder-stack-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-stack-open.svg"
        )),
        "folder-stencil" => Some(include_bytes!("../../assets/file_icons/folder-stencil.svg")),
        "folder-stencil-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-stencil-open.svg"
        )),
        "folder-store" => Some(include_bytes!("../../assets/file_icons/folder-store.svg")),
        "folder-store-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-store-open.svg"
        )),
        "folder-storybook" => Some(include_bytes!(
            "../../assets/file_icons/folder-storybook.svg"
        )),
        "folder-storybook-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-storybook-open.svg"
        )),
        "folder-stylus" => Some(include_bytes!("../../assets/file_icons/folder-stylus.svg")),
        "folder-stylus-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-stylus-open.svg"
        )),
        "folder-sublime" => Some(include_bytes!("../../assets/file_icons/folder-sublime.svg")),
        "folder-sublime-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-sublime-open.svg"
        )),
        "folder-supabase" => Some(include_bytes!(
            "../../assets/file_icons/folder-supabase.svg"
        )),
        "folder-supabase-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-supabase-open.svg"
        )),
        "folder-svelte" => Some(include_bytes!("../../assets/file_icons/folder-svelte.svg")),
        "folder-svelte-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-svelte-open.svg"
        )),
        "folder-svg" => Some(include_bytes!("../../assets/file_icons/folder-svg.svg")),
        "folder-svg-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-svg-open.svg"
        )),
        "folder-syntax" => Some(include_bytes!("../../assets/file_icons/folder-syntax.svg")),
        "folder-syntax-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-syntax-open.svg"
        )),
        "folder-target" => Some(include_bytes!("../../assets/file_icons/folder-target.svg")),
        "folder-target-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-target-open.svg"
        )),
        "folder-taskfile" => Some(include_bytes!(
            "../../assets/file_icons/folder-taskfile.svg"
        )),
        "folder-taskfile-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-taskfile-open.svg"
        )),
        "folder-tasks" => Some(include_bytes!("../../assets/file_icons/folder-tasks.svg")),
        "folder-tasks-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-tasks-open.svg"
        )),
        "folder-television" => Some(include_bytes!(
            "../../assets/file_icons/folder-television.svg"
        )),
        "folder-television-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-television-open.svg"
        )),
        "folder-temp" => Some(include_bytes!("../../assets/file_icons/folder-temp.svg")),
        "folder-temp-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-temp-open.svg"
        )),
        "folder-template" => Some(include_bytes!(
            "../../assets/file_icons/folder-template.svg"
        )),
        "folder-template-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-template-open.svg"
        )),
        "folder-terraform" => Some(include_bytes!(
            "../../assets/file_icons/folder-terraform.svg"
        )),
        "folder-terraform-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-terraform-open.svg"
        )),
        "folder-test" => Some(include_bytes!("../../assets/file_icons/folder-test.svg")),
        "folder-test-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-test-open.svg"
        )),
        "folder-theme" => Some(include_bytes!("../../assets/file_icons/folder-theme.svg")),
        "folder-theme-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-theme-open.svg"
        )),
        "folder-toc" => Some(include_bytes!("../../assets/file_icons/folder-toc.svg")),
        "folder-toc-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-toc-open.svg"
        )),
        "folder-tools" => Some(include_bytes!("../../assets/file_icons/folder-tools.svg")),
        "folder-tools-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-tools-open.svg"
        )),
        "folder-trash" => Some(include_bytes!("../../assets/file_icons/folder-trash.svg")),
        "folder-trash-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-trash-open.svg"
        )),
        "folder-trigger" => Some(include_bytes!("../../assets/file_icons/folder-trigger.svg")),
        "folder-trigger-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-trigger-open.svg"
        )),
        "folder-turborepo" => Some(include_bytes!(
            "../../assets/file_icons/folder-turborepo.svg"
        )),
        "folder-turborepo-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-turborepo-open.svg"
        )),
        "folder-typescript" => Some(include_bytes!(
            "../../assets/file_icons/folder-typescript.svg"
        )),
        "folder-typescript-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-typescript-open.svg"
        )),
        "folder-ui" => Some(include_bytes!("../../assets/file_icons/folder-ui.svg")),
        "folder-ui-open" => Some(include_bytes!("../../assets/file_icons/folder-ui-open.svg")),
        "folder-unity" => Some(include_bytes!("../../assets/file_icons/folder-unity.svg")),
        "folder-unity-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-unity-open.svg"
        )),
        "folder-update" => Some(include_bytes!("../../assets/file_icons/folder-update.svg")),
        "folder-update-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-update-open.svg"
        )),
        "folder-upload" => Some(include_bytes!("../../assets/file_icons/folder-upload.svg")),
        "folder-upload-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-upload-open.svg"
        )),
        "folder-utils" => Some(include_bytes!("../../assets/file_icons/folder-utils.svg")),
        "folder-utils-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-utils-open.svg"
        )),
        "folder-vercel" => Some(include_bytes!("../../assets/file_icons/folder-vercel.svg")),
        "folder-vercel-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-vercel-open.svg"
        )),
        "folder-verdaccio" => Some(include_bytes!(
            "../../assets/file_icons/folder-verdaccio.svg"
        )),
        "folder-verdaccio-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-verdaccio-open.svg"
        )),
        "folder-video" => Some(include_bytes!("../../assets/file_icons/folder-video.svg")),
        "folder-video-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-video-open.svg"
        )),
        "folder-views" => Some(include_bytes!("../../assets/file_icons/folder-views.svg")),
        "folder-views-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-views-open.svg"
        )),
        "folder-vm" => Some(include_bytes!("../../assets/file_icons/folder-vm.svg")),
        "folder-vm-open" => Some(include_bytes!("../../assets/file_icons/folder-vm-open.svg")),
        "folder-vscode" => Some(include_bytes!("../../assets/file_icons/folder-vscode.svg")),
        "folder-vscode-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-vscode-open.svg"
        )),
        "folder-vue" => Some(include_bytes!("../../assets/file_icons/folder-vue.svg")),
        "folder-vue-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-vue-open.svg"
        )),
        "folder-vuepress" => Some(include_bytes!(
            "../../assets/file_icons/folder-vuepress.svg"
        )),
        "folder-vuepress-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-vuepress-open.svg"
        )),
        "folder-wakatime" => Some(include_bytes!(
            "../../assets/file_icons/folder-wakatime.svg"
        )),
        "folder-wakatime-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-wakatime-open.svg"
        )),
        "folder-webpack" => Some(include_bytes!("../../assets/file_icons/folder-webpack.svg")),
        "folder-webpack-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-webpack-open.svg"
        )),
        "folder-windows" => Some(include_bytes!("../../assets/file_icons/folder-windows.svg")),
        "folder-windows-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-windows-open.svg"
        )),
        "folder-wordpress" => Some(include_bytes!(
            "../../assets/file_icons/folder-wordpress.svg"
        )),
        "folder-wordpress-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-wordpress-open.svg"
        )),
        "folder-yarn" => Some(include_bytes!("../../assets/file_icons/folder-yarn.svg")),
        "folder-yarn-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-yarn-open.svg"
        )),
        "folder-zeabur" => Some(include_bytes!("../../assets/file_icons/folder-zeabur.svg")),
        "folder-zeabur-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-zeabur-open.svg"
        )),
        "folder-zed" => Some(include_bytes!("../../assets/file_icons/folder-zed.svg")),
        "folder-zed-open" => Some(include_bytes!(
            "../../assets/file_icons/folder-zed-open.svg"
        )),
        "font" => Some(include_bytes!("../../assets/file_icons/font.svg")),
        "forth" => Some(include_bytes!("../../assets/file_icons/forth.svg")),
        "fortran" => Some(include_bytes!("../../assets/file_icons/fortran.svg")),
        "foxpro" => Some(include_bytes!("../../assets/file_icons/foxpro.svg")),
        "freemarker" => Some(include_bytes!("../../assets/file_icons/freemarker.svg")),
        "fsharp" => Some(include_bytes!("../../assets/file_icons/fsharp.svg")),
        "fusebox" => Some(include_bytes!("../../assets/file_icons/fusebox.svg")),
        "gamemaker" => Some(include_bytes!("../../assets/file_icons/gamemaker.svg")),
        "garden" => Some(include_bytes!("../../assets/file_icons/garden.svg")),
        "gatsby" => Some(include_bytes!("../../assets/file_icons/gatsby.svg")),
        "gcp" => Some(include_bytes!("../../assets/file_icons/gcp.svg")),
        "gemfile" => Some(include_bytes!("../../assets/file_icons/gemfile.svg")),
        "gemini" => Some(include_bytes!("../../assets/file_icons/gemini.svg")),
        "gemini-ai" => Some(include_bytes!("../../assets/file_icons/gemini-ai.svg")),
        "git" => Some(include_bytes!("../../assets/file_icons/git.svg")),
        "github-sponsors" => Some(include_bytes!(
            "../../assets/file_icons/github-sponsors.svg"
        )),
        "gitlab" => Some(include_bytes!("../../assets/file_icons/gitlab.svg")),
        "gitpod" => Some(include_bytes!("../../assets/file_icons/gitpod.svg")),
        "gleam" => Some(include_bytes!("../../assets/file_icons/gleam.svg")),
        "gnuplot" => Some(include_bytes!("../../assets/file_icons/gnuplot.svg")),
        "go" => Some(include_bytes!("../../assets/file_icons/go.svg")),
        "go-mod" => Some(include_bytes!("../../assets/file_icons/go-mod.svg")),
        "godot" => Some(include_bytes!("../../assets/file_icons/godot.svg")),
        "godot-assets" => Some(include_bytes!("../../assets/file_icons/godot-assets.svg")),
        "google" => Some(include_bytes!("../../assets/file_icons/google.svg")),
        "gradle" => Some(include_bytes!("../../assets/file_icons/gradle.svg")),
        "grafana-alloy" => Some(include_bytes!("../../assets/file_icons/grafana-alloy.svg")),
        "grain" => Some(include_bytes!("../../assets/file_icons/grain.svg")),
        "graphcool" => Some(include_bytes!("../../assets/file_icons/graphcool.svg")),
        "graphql" => Some(include_bytes!("../../assets/file_icons/graphql.svg")),
        "gridsome" => Some(include_bytes!("../../assets/file_icons/gridsome.svg")),
        "groovy" => Some(include_bytes!("../../assets/file_icons/groovy.svg")),
        "grunt" => Some(include_bytes!("../../assets/file_icons/grunt.svg")),
        "gsh-notebook" => Some(include_bytes!("../../assets/file_icons/gsh-notebook.svg")),
        "gsh-panel" => Some(include_bytes!("../../assets/file_icons/gsh-panel.svg")),
        _ => None,
    }
}
