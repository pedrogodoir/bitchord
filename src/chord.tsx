import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

interface NodeInfo {
  id: number;
  address: string;
  successor_id: number;
  successor_address: string;
  predecessor_id: number | null;
  predecessor_address: string | null;
}

interface ChordFile {
  file_name: string;
  total_size: number;
  file_hash: string;
  routing_id: number;
  chunk_hashes: string[];
}

const roleStyles: Record<string, { dot: string; badge: string }> = {
  Sucessor: {
    dot: "bg-emerald-500 shadow-[0_0_6px_#10b981]",
    badge: "text-emerald-400 bg-emerald-500/10 border-emerald-500/20",
  },
  Predecessor: {
    dot: "bg-blue-500 shadow-[0_0_6px_#3b82f6]",
    badge: "text-blue-400 bg-blue-500/10 border-blue-500/20",
  },
};


function formatBytes(bytes: number) {
  if (bytes === 0) return '0 Bytes';
  const k = 1024;
  const sizes = ['Bytes', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

export default function ChordDashboard() {
  const [node, setNode] = useState<NodeInfo | null>(null);
  const [hasLeft, setHasLeft] = useState<boolean>(false);
  const [expandedNodes, setExpandedNodes] = useState<Record<number, boolean>>({});
  
  // Estado para controlar a aba ativa
  const [activeTab, setActiveTab] = useState<"network" | "files">("network");
  
  // Estado para armazenar os arquivos disponíveis
  const [availableFiles, setAvailableFiles] = useState<ChordFile[]>([]);

  const fetchData = async () => {
    try {
      const data = await invoke<NodeInfo>("get_node_info");
      setNode(data); 
    } catch (error) {
      console.error("Erro ao buscar informações do nó:", error);
    }
  };

  // Função para buscar os arquivos da rede (melhorar)
  const fetchFiles = async () => {
    try {
      // Chama o Rust e espera um Array de ChordFile (TorrentMeta)
      const files = await invoke<ChordFile[]>("get_all_files_network");
      setAvailableFiles(files);
    } catch (error) {
      console.error("Erro ao buscar arquivos na rede:", error);
      // talvez mostrar um toast?
    }
  };

  useEffect(() => {
    fetchData();
    const interval = setInterval(fetchData, 2000);
    return () => clearInterval(interval);
  }, [hasLeft]);

  // Busca arquivos sempre que a aba de arquivos for aberta
  useEffect(() => {
    if (activeTab === "files") {
      fetchFiles();
    }
  }, [activeTab]);

  const handlePowerOff = async () => {
    try {
      await invoke("leave_network");
      setHasLeft(true);
    } catch (e) {
      console.error("Erro ao tentar sair da rede:", e);
    }
  };

  const handlePowerOn = async () => {
    try {
      await invoke("join_network");
      setHasLeft(false);
      await fetchData();
    } catch (e) {
      console.error("Erro ao tentar voltar à rede:", e);
    }
  };

  const toggleNode = (nodeId: number) => {
    setExpandedNodes(prev => ({
      ...prev,
      [nodeId]: !prev[nodeId]
    }));
  };

  if (hasLeft) {
    return (
      <div className="h-full w-full flex flex-col items-center justify-center bg-[#1e1f22] text-white">
          <div 
            onClick={handlePowerOn}
            className="w-16 h-16 bg-gradient-to-b from-rose-500 to-rose-600 hover:from-emerald-500 hover:to-emerald-600 rounded-md flex items-center justify-center shadow-[0_0_25px_rgba(244,63,94,0.4)] hover:shadow-[0_0_25px_rgba(16,185,129,0.4)] border border-rose-400/20 hover:border-emerald-400/30 cursor-pointer transition-all active:scale-95 group"
            title="Ligar e juntar ao Anel"
          >
            <svg className="w-8 h-8 text-white group-hover:animate-none animate-pulse" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2.5" d="M13 10V3L4 14h7v7l9-11h-7z" />
            </svg>
          </div>
      </div>
    );
  }

  if (!node) return (
    <div className="h-full w-full flex flex-col items-center justify-center bg-[#1e1f22] text-zinc-400 font-mono text-xs">
      <div className="w-6 h-6 border-2 border-blue-500 border-t-transparent rounded-full animate-spin mb-3"></div>
      <span>AGUARDANDO CONTATO COM O CORPO DO NÓ...</span>
    </div>
  );

  const knownMembers = [];

  if (node.successor_id !== node.id) {
    knownMembers.push({
      role: "Sucessor",
      id: node.successor_id,
      address: node.successor_address,
    });
  }

  if (node.predecessor_id !== null && node.predecessor_address !== null) {
    if (node.predecessor_id !== node.id && node.predecessor_id !== node.successor_id) {
      knownMembers.push({
        role: "Predecessor",
        id: node.predecessor_id,
        address: node.predecessor_address,
      });
    }
  }

  return (
    <div className="h-full w-full flex flex-col bg-[#1e1f22] text-zinc-200 font-sans select-none overflow-hidden text-sm">
      
      {/* Painel do Host */}
      <div className="p-5 bg-[#232428] border-b border-zinc-800 flex items-center justify-between shrink-0 z-10">
        <div className="flex items-center gap-4">
          <div 
            onClick={handlePowerOff}
            className="w-12 h-12 bg-gradient-to-b from-emerald-500 to-emerald-600 hover:from-rose-500 hover:to-rose-600 rounded-md flex items-center justify-center shadow-lg border border-emerald-400/20 hover:border-rose-400/30 cursor-pointer transition-all active:scale-95 group"
            title="Clique para desconectar do Anel"
          >
            <svg className="w-6 h-6 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2.5" d="M13 10V3L4 14h7v7l9-11h-7z" />
            </svg>
          </div>
          
          <div>
            <div className="flex items-center gap-2">
              <span className="text-xs uppercase tracking-wider font-bold text-zinc-500">My ID:</span>
              <span className="bg-zinc-800 px-1.5 py-0.5 rounded text-xs font-mono font-bold text-emerald-500 border border-zinc-700 shadow-inner">
                {node.id}
              </span>
            </div>
            <div className="text-2xl font-mono font-bold tracking-tight text-white mt-0.5">
              {node.address}
            </div>
          </div>
        </div>
      </div>

      {/* Navegação por Abas (TABS) */}
      <div className="flex border-b border-zinc-800 bg-[#232428] px-5 shrink-0 z-10 shadow-sm">
        <button
          onClick={() => setActiveTab("network")}
          className={`py-3 px-4 text-sm font-semibold border-b-2 transition-all duration-200 ${
            activeTab === "network" 
              ? "border-emerald-500 text-emerald-400" 
              : "border-transparent text-zinc-500 hover:text-zinc-300 hover:border-zinc-700"
          }`}
        >
          Rede P2P
        </button>
        <button
          onClick={() => setActiveTab("files")}
          className={`py-3 px-4 text-sm font-semibold border-b-2 transition-all duration-200 ${
            activeTab === "files" 
              ? "border-blue-500 text-blue-400" 
              : "border-transparent text-zinc-500 hover:text-zinc-300 hover:border-zinc-700"
          }`}
        >
          Arquivos da Rede
        </button>
      </div>

      {/* Conteúdo Dinâmico (Condicional) */}
      <div className="p-4 bg-[#1e1f22] flex-1 flex flex-col overflow-y-auto">
        
        {/* ABA DE REDE*/}
        {activeTab === "network" && (
          <>
            {/* Cabeçalho da Lista */}
            <div className="flex items-center gap-3 px-2 py-2 mb-3 rounded shrink-0">
              <svg className="w-5 h-5 text-blue-400" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <rect x="16" y="16" width="6" height="6" rx="1"/>
                <rect x="2" y="16" width="6" height="6" rx="1"/>
                <rect x="9" y="2" width="6" height="6" rx="1"/>
                <path d="M5 16v-3a1 1 0 0 1 1-1h12a1 1 0 0 1 1 1v3"/>
                <path d="M12 12V8"/>
              </svg>
              <span className="font-semibold text-zinc-200">Rede DHT (Mód. 256)</span>
              <span className="text-xs text-zinc-500 font-mono ml-auto bg-zinc-900 px-2 py-1 rounded border border-zinc-800">
                {knownMembers.length} Conexões Mapeadas
              </span>
            </div>

            {/* Lista Dinâmica */}
            <div className="pl-6 pr-2 space-y-3 border-l border-zinc-800/60 ml-4 flex-1">
              {knownMembers.map((member) => { 
                const style = roleStyles[member.role] || roleStyles.Sucessor;
                const isExpanded = !!expandedNodes[member.id];
                
                return (
                  <div key={member.id} className="flex flex-col bg-zinc-800/20 border border-zinc-800/40 rounded-lg overflow-hidden transition-all">
                    
                    {/* Área de Informação Principal */}
                    <div 
                      onClick={() => toggleNode(member.id)}
                      className="flex items-center justify-between p-3 hover:bg-zinc-800/50 cursor-pointer transition-colors group select-none"
                    >
                      <div className="flex items-center gap-3">
                        <span className={`w-2.5 h-2.5 rounded-full ${style.dot}`}></span>
                        <span className="font-mono text-zinc-200 font-bold text-base">ID {member.id}</span>
                        <span className={`text-[10px] uppercase tracking-wider px-2 py-0.5 rounded border font-bold ${style.badge}`}>
                          {member.role}
                        </span>
                        
                        <svg 
                          className={`w-3.5 h-3.5 text-zinc-500 transition-transform duration-200 ${isExpanded ? 'rotate-180' : ''}`} 
                          fill="none" 
                          stroke="currentColor" 
                          viewBox="0 0 24 24"
                        >
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2.5" d="M19 9l-7 7-7-7" />
                        </svg>
                      </div>
                      
                      <div className="text-sm font-mono text-zinc-500 group-hover:text-zinc-300 transition-colors">
                        {member.address}
                      </div>
                    </div>

                    {/* Gaveta Oculta (Collapser) */}
                    {isExpanded && (
                      <div className="p-3 bg-zinc-900/40 border-t border-zinc-800/50 flex flex-col gap-2 transition-all">
                        <button 
                          className="bg-zinc-800 hover:bg-zinc-700 text-zinc-300 py-2 px-4 rounded text-xs font-semibold transition-colors border border-zinc-700 hover:border-blue-500/50"
                          onClick={async ()=>{
                            const file = await open({
                              multiple: false,
                              directory: false,
                            });
                            console.log(file);
                            if(file) await invoke('upload_file',{file});
                          }}
                        >
                          Upload File para este Nó
                        </button>
                      </div>
                    )}
                  </div>
                )
              })}

              {node.predecessor_id === null && (
                <div className="flex items-center justify-between p-3 bg-zinc-900/30 border border-zinc-800/30 rounded-lg border-dashed">
                    <div className="flex items-center gap-3">
                      <span className="w-2.5 h-2.5 rounded-full bg-zinc-700 animate-pulse"></span>
                      <span className="font-mono text-zinc-600 italic text-sm">Procurando predecessor...</span>
                    </div>
                </div>
              )}
            </div>
          </>
        )}

        {/* ======== ABA DE ARQUIVOS DISPONÍVEIS ======== */}
        {activeTab === "files" && (
          <div className="flex-1 flex flex-col">
            <div className="flex items-center gap-3 px-2 py-2 mb-3 rounded shrink-0">
              <svg className="w-5 h-5 text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" />
              </svg>
              <span className="font-semibold text-zinc-200">Arquivos no Anel</span>
              <button 
                onClick={fetchFiles}
                className="ml-auto text-xs bg-zinc-800 hover:bg-zinc-700 text-zinc-300 px-3 py-1.5 rounded transition-colors border border-zinc-700"
              >
                Atualizar Lista
              </button>
            </div>

            {availableFiles.length === 0 ? (
              <div className="flex-1 flex items-center justify-center text-zinc-600 font-mono text-xs italic">
                Nenhum arquivo indexado na rede no momento.
              </div>
            ) : (
              <div className="grid grid-cols-1 gap-2">
                {availableFiles.map((file) => (
                  // Usamos o file_hash como chave única (key)
                  <div key={file.file_hash} className="flex items-center justify-between bg-zinc-800/30 border border-zinc-800/50 p-3 rounded-lg hover:bg-zinc-800/60 transition-colors">
                    <div className="flex flex-col">
                      <span className="text-zinc-200 font-medium">{file.file_name}</span>
                      <span className="text-xs text-zinc-500 font-mono mt-0.5">
                        Tamanho: {formatBytes(file.total_size)} | Pedaços: {file.chunk_hashes.length}
                      </span>
                    </div>
                    <div className="flex items-center gap-4">
                      <span className="text-[10px] text-zinc-400 bg-zinc-900 px-2 py-1 rounded border border-zinc-800" title={file.file_hash}>
                        ID no Chord: <span className="text-blue-500 font-bold ml-1">{file.routing_id}</span>
                      </span>
                      <button 
                        className="text-blue-400 hover:text-blue-300 text-xs font-semibold px-2"
                        onClick={() => console.log("Download de:", file.file_name)}
                      >
                        Baixar
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </div>
      
      {/* Footer */}
      <div className="bg-[#151618] border-t border-zinc-800 p-4 shrink-0 shadow-[0_-10px_20px_rgba(0,0,0,0.2)] z-10">
        <div className="text-center text-[10px] text-zinc-600 font-mono tracking-wide">
          {activeTab === "network" 
            ? "Selecione e expanda um nó ativo acima na lista da rede para fragmentar e publicar arquivos no Chord Network."
            : "Visualize os arquivos distribuídos através da tabela Hash Distribuída (DHT)."}
        </div>
      </div>
      
    </div>
  );
}