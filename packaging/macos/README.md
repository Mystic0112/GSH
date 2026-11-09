# Empacotamento para macOS

`build-app.sh` monta o `GSH.app` (e, com `--dmg`, o DMG) a partir dos binários em Rust. Não usa nenhuma ferramenta além das que vêm com o macOS e o Xcode (`codesign`, `sips`, `iconutil`, `hdiutil`, `plutil`).

```sh
packaging/macos/build-app.sh                 # target/macos/GSH.app, assinatura ad-hoc
packaging/macos/build-app.sh --dmg           # e target/macos/GSH-<versão>-macos-<arch>.dmg
packaging/macos/build-app.sh --skip-build    # reaproveita target/release
```

## O que entra no bundle

```
GSH.app/Contents/
  Info.plist              # gerado de Info.plist.in
  MacOS/GSH               # o gsh-app
  Resources/AppIcon.icns  # gerado de AppIcon-1024.png
  Resources/bin/gsh       # a CLI
```

O executável do app se chama `GSH` e a CLI vai em `Resources/bin` porque o macOS é case-insensitive: `MacOS/GSH` e `MacOS/gsh` seriam o mesmo arquivo.

O `Info.plist` tem o identificador `work.gorila.gsh` (sobrescreva com `BUNDLE_ID=...`), a versão do `[workspace.package]` do `Cargo.toml`, o número de build (`BUILD_NUMBER`, padrão: contagem de commits) e a versão mínima do macOS lida do próprio binário. A chave `NSLocalNetworkUsageDescription` é necessária: no macOS 15 ou mais novo, sem ela o sistema pode bloquear a rede local (a fazenda do time) sem perguntar.

O binário só depende de frameworks do sistema, então não há `.dylib` de terceiros para embutir.

## Assinatura

- **Ad-hoc (padrão):** serve para rodar no Mac em que foi gerado. Em outro Mac o Gatekeeper bloqueia a primeira abertura de um arquivo baixado (clique com o botão direito em Abrir, ou `xattr -dr com.apple.quarantine GSH.app`).
- **Developer ID:** `build-app.sh --sign "Developer ID Application: Nome (TEAMID)" --dmg`. Assina com hardened runtime e timestamp, o que a notarização exige.

## Notarização (manual, com conta de desenvolvedor)

Depois de assinar com Developer ID:

```sh
xcrun notarytool store-credentials gsh-notary --apple-id <email> --team-id <TEAMID>   # uma vez
xcrun notarytool submit target/macos/GSH-*.dmg --keychain-profile gsh-notary --wait
xcrun stapler staple target/macos/GSH-*.dmg
```

A CI ainda não assina nem notariza: faltam o certificado e as credenciais como segredos do repositório.

## Ainda não coberto

- Binário universal (`aarch64` e `x86_64`). Hoje sai só da arquitetura da máquina.
- Atualização automática (Sparkle) e `appcast`.
- Localização do `Info.plist` (o texto da rede local está em português).
