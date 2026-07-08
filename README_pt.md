# BitChord

Um aplicativo de compartilhamento de arquivos peer-to-peer inspirado no BitTorrent, construído sobre o protocolo de Tabela Hash Distribuída (DHT) Chord.

## 🧠 Visão Geral da Arquitetura

A arquitetura do sistema é descentralizada e dividida em duas camadas principais de responsabilidade:

### 1. A Tabela Hash Distribuída (Protocolo Chord)
A rede opera sob uma topologia de anel lógico com um espaço de identificação de 8 bits (IDs de 0 a 255).
* **Nós (Nodes):** Cada instância do aplicativo atua como um nó (`Node`) com um ID gerado a partir do seu endereço IP. Cada nó mantém referências para seu Sucessor, seu Predecessor e uma **Finger Table** (tabela de roteamento) para garantir uma complexidade de busca logarítmica.
* **Armazenamento de Metadados:** O anel Chord **não** armazena os arquivos pesados. Ele atua como um rastreador descentralizado (tracker), armazenando apenas os metadados dos torrents (`TorrentMeta`), mapeados pelo hash SHA-1 do nome do arquivo.

### 2. A Camada de Compartilhamento de Arquivos
* **Seeders & Chunks (Pedaços):** Os arquivos reais permanecem nos discos dos usuários. Durante o upload, os arquivos são divididos em pedaços fixos de 256 KB (`CHUNK_SIZE`).
* **Seeding Automático:** Assim que um nó baixa um arquivo com sucesso, ele se registra automaticamente como um seeder na DHT, fortalecendo a rede.
* **Arquivos `.bitchord`:** Funcionam como os tradicionais arquivos `.torrent`. Eles contêm as informações vitais (hashes dos blocos, tamanho total, nome do arquivo) necessárias para que um cliente solicite pedaços e valide a integridade dos dados.

---

## 🔄 Fluxos Principais de Execução

### Fluxo de Descoberta e Entrada
1. Um nó entra na rede e emite um ping UDP Multicast (`PING_CHORD_JOIN`) na porta 5000.
2. Se um nó existente estiver escutando, ele confirma a presença.
3. O nó ingressante se conecta via TCP ao nó de contato e usa o RPC `FindSuccessor` para descobrir sua posição correta no anel de 0 a 255.
4. Rotinas em background (`stabilize` e `fix_fingers`) rodam periodicamente para costurar o nó na rede e atualizar as tabelas de roteamento.

### Fluxo de Upload / Semeadura
1. O usuário seleciona um arquivo através da interface (`upload_file`).
2. O backend lê o arquivo, divide-o em blocos de 256 KB e calcula o hash SHA-1 para cada bloco.
3. O hash SHA-1 do *nome* do arquivo dita qual ID do anel é o responsável por armazenar este metadado.
4. O sistema roteia a requisição (`find_successor_rpc`) para o nó responsável e envia o `TorrentMeta` via uma mensagem `PutData`. O IP de quem fez o upload é adicionado à lista de `seeders`.
5. Um arquivo `.bitchord` é gerado localmente e o nó original começa a semear.

### Fluxo de Busca e Download (Paralelo e Balanceado)
1. O usuário insere um arquivo `.bitchord` ou um hash direto. O sistema extrai o ID alvo.
2. A rede é consultada (`fetch_meta`) para encontrar o nó que detém o `TorrentMeta`.
3. O nó inicia um **download paralelo** (`download_all_chunks`) usando um pool de threads.
4. **Balanceamento de Carga:** As requisições de pedaços são distribuídas uniformemente entre todos os seeders disponíveis usando uma estratégia Round-Robin, evitando gargalos na rede.
5. À medida que os pedaços chegam simultaneamente, seus hashes SHA-1 são recalculados e verificados. Pedaços válidos são remontados na ordem correta.
6. O nó se registra como um novo seeder para o arquivo baixado.

### Tolerância a Falhas e Recuperação
* **Protocolo Heartbeat:** Os nós monitoram ativamente seus vizinhos imediatos usando verificações rápidas de ping TCP (`is_alive`).
* Se um nó falhar ou cair inesperadamente (ex: fechamento forçado), os nós vizinhos detectam o timeout, ignoram o nó morto dinamicamente usando suas Finger Tables e curam o anel automaticamente.

### Fluxo de Saída Graciosa
1. Quando o aplicativo é fechado normalmente (`leave_network`), o nó aciona a função `leave_ring`.
2. **Limpeza de Seeders Fantasmas:** O nó consulta ativamente a DHT para remover seu próprio IP das listas de seeders de todos os arquivos que estava compartilhando.
3. Ele transfere todo o seu dicionário de metadados (`storage`) para o seu Sucessor para evitar perda de dados na rede.
4. Ele notifica seu Predecessor e Sucessor para se conectarem um ao outro, fechando perfeitamente a lacuna no anel.

---

## 🔌 Interfaces (APIs)

### Interface de Rede Nó a Nó
Toda a comunicação TCP entre os pares usa objetos serializados em JSON modelados pelo enum `Message`:
* **Manutenção do Chord:** `Ping`, `Pong`, `FindSuccessor`, `GetPredecessor`, `Notify`.
* **Roteamento e Armazenamento:** `PutData`, `GetData`, `TransferKeys`.
* **Transferência de Arquivos:** `RequestChunk`, `ChunkData`.

### Interface Backend para Frontend (Tauri)
Comandos expostos pelo Rust para a interface React:
* `get_node_info`: Retorna o estado atual do nó (ID, sucessor, predecessor).
* `join_network` / `leave_network`: Controla a entrada e a saída graciosa do anel.
* `upload_file`: Inicia o processo de publicação do arquivo e retorna mensagens de status.
* `search_file`: Consulta a DHT por metadados.
* `download_file` / `download_by_hash`: Inicia o processo de coleta de pedaços.

---

## 💻 Tecnologias Utilizadas

[![Static Badge](https://img.shields.io/badge/Tauri-000000?style=flat&logo=Tauri)](https://v2.tauri.app/)
[![Static Badge](https://img.shields.io/badge/Rust-000000?style=flat&logo=Rust)](https://rust-lang.org/)
[![Static Badge](https://img.shields.io/badge/React-000000?style=flat&logo=React)](https://react.dev/)

* **Linguagem Principal:** Rust (Segurança de memória, alta concorrência).
* **Rede:** TCP padrão para RPCs confiáveis e UDP Multicast para descoberta em rede local sem tracker.
* **Concorrência:** `rayon` para paralelismo de dados e gerenciamento de pool de threads durante os downloads.
* **Criptografia:** SHA-1 para geração de IDs e validação de integridade dos pedaços.
* **Serialização:** JSON via `serde` / `serde_json`.

---

## ▶️ Compilação e Execução

Existem duas maneiras de compilar o BitChord, dependendo do seu ambiente e sistema operacional.

### Método 1: Com Docker (Apenas para Linux / Gera `.AppImage`)
Ideal se você estiver no Linux e **não** quiser instalar o ambiente Rust e Node.js na sua máquina. O Docker cuidará de tudo e gerará um executável autossuficiente.

### 1. Compilar a imagem builder do Tauri:
Na raiz do projeto, execute:
```bash
docker build -f Dockerfile.build -t tauri-builder .
```

### 2. Iniciar o contêiner de compilação

```bash
docker run --rm -it -v "${PWD}:/app" tauri-builder
```

Você verá um prompt semelhante a:

```text
root@xxxxxxxx:/app#
```

Isso indica que você está dentro do contêiner.

### 3. Instalar dependências e compilar

Dentro do contêiner, execute:

```bash
npm install
npm run tauri build
```

### 4. Localizar e executar o aplicativo

O AppImage gerado estará localizado em:

```text
src-tauri/target/release/bundle/appimage/
```

Conceda permissão de execução e execute o arquivo:

```bash
chmod +x bitchord_0.1.0_amd64.AppImage
./bitchord_0.1.0_amd64.AppImage
```

## Método 2: Nativo (Windows ou Linux com Rust já instalado)

Ideal se você já possui o ambiente de desenvolvimento configurado, com `Node.js` e `Rust` instalados. O Tauri detecta automaticamente o sistema operacional e gera o executável correspondente (`.exe` para Windows ou `.AppImage`/`.deb` para Linux).

### 1. Instalar as dependências do frontend

Na raiz do projeto, abra o terminal e execute:

```bash
npm install
```

### 2. Compilar a aplicação

Use o script do NPM que chama o Tauri para gerar o executável otimizado:

```bash
npm run tauri build
```

### 3. Localizar o executável final

**No Windows:** o instalador estará em:

```text
src-tauri/target/release/bundle/nsis/
```

O executável direto também pode ser encontrado em:

```text
src-tauri/target/release/
```

**No Linux:** o AppImage estará em:

```text
src-tauri/target/release/bundle/appimage/
```
