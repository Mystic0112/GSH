# Editor de código

## Contexto

Arquivos de texto de até 2 MiB precisam de edição com histórico, realce léxico e proteção contra alterações externas.

## Decisão

O buffer usa `String` UTF-8 e histórico de diffs, agrupando digitação contínua e limitando o histórico a 500 passos. O syntect identifica escopos léxicos, mapeados para tokens GitHub Dark na interface; acima de 512 KiB o editor usa texto puro. Antes de salvar, compara tamanho e data de modificação com o arquivo carregado; conflito exige confirmação explícita para sobrescrever. A gravação usa arquivo temporário no mesmo diretório, rename e sincronização do diretório pai.

## Consequências

O histórico não copia o documento inteiro por tecla, mas alterações grandes ainda ocupam memória. O realce é recalculado integralmente abaixo do limite e pode custar tempo; o syntect aumenta o binário, custo ainda não medido. A comparação de metadados não detecta todas as alterações possíveis nem elimina a corrida entre checagem e rename.
