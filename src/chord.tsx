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

export default function ChordDashboard() {
  const [node, setNode] = useState<NodeInfo | null>(null);
  const [hasLeft, setHasLeft] = useState<boolean>(false);
  
  const fetchData = async () => {
    try {
      const data = await invoke<NodeInfo>("get_node_info");
      setNode(data); 
    } catch (error) {
      console.error("Erro ao buscar informações do nó:", error);
    }
  };

  useEffect(() => {
    fetchData();
    const interval = setInterval(fetchData, 2000);
    return () => clearInterval(interval);
  }, [hasLeft]);

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

  // Botão de upload de arquivos
  const handleUpload = async () => {
    try {
      const file = await open({
        multiple: false,
        directory: false,
      });
      if (file) {
        await invoke('upload_file', { file });
      }
    } catch (e) {
      console.error("Erro no upload:", e);
    }
  };

  // Função para carregar um .bitchord e iniciar o download
  const handleLoadBitchord = async () => {
    try {
      const file = await open({
        multiple: false,
        directory: false,
        filters: [{ name: 'BitChord Files', extensions: ['bitchord'] }] // Filtra para mostrar só .bitchord
      });
      
      if (file) {
        console.log("Arquivo .bitchord selecionado:", file);
        // Descomente e ajuste a linha abaixo conforme o comando no Rust
        await invoke('download_file', { filepath: file });
      }
    } catch (e) {
      console.error("Erro ao abrir arquivo .bitchord:", e);
    }
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

        {/* Agrupamento dos Botões de Ação */}
        <div className="flex items-center gap-3">
          {/* Botão de Carregar .bitchord */}
          <button
            onClick={handleLoadBitchord}
            className="flex items-center gap-2 bg-emerald-600 hover:bg-emerald-500 text-white px-4 py-2.5 rounded-md text-sm font-bold transition-all shadow-[0_0_15px_rgba(16,185,129,0.2)] hover:shadow-[0_0_20px_rgba(16,185,129,0.4)] border border-emerald-500 hover:scale-105 active:scale-95"
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2.5" d="M3 15v4c0 1.1.9 2 2 2h14a2 2 0 002-2v-4M17 9l-5 5-5-5M12 12.8V2.5" />
            </svg>
            Abrir .bitchord
          </button>

          {/* Botão de Upload */}
          <button
            onClick={handleUpload}
            className="flex items-center gap-2 bg-blue-600 hover:bg-blue-500 text-white px-4 py-2.5 rounded-md text-sm font-bold transition-all shadow-[0_0_15px_rgba(37,99,235,0.2)] hover:shadow-[0_0_20px_rgba(37,99,235,0.4)] border border-blue-500 hover:scale-105 active:scale-95"
          >
            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2.5" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12" />
            </svg>
            Publicar Arquivo
          </button>
        </div>
      </div>

      {/* Navegação  */}
      <div className="flex border-b border-zinc-800 bg-[#232428] px-5 shrink-0 z-10 shadow-sm">
        <div className="py-3 px-4 text-sm font-semibold border-b-2 border-emerald-500 text-emerald-400">
          Rede P2P
        </div>
      </div>

      {/* Conteúdo Dinâmico */}
      <div className="p-4 bg-[#1e1f22] flex-1 flex flex-col overflow-y-auto">
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

        <div className="pl-6 pr-2 space-y-3 border-l border-zinc-800/60 ml-4 flex-1">
          {knownMembers.map((member) => { 
            const style = roleStyles[member.role] || roleStyles.Sucessor;
            
            return (
              <div key={member.id} className="flex flex-col bg-zinc-800/20 border border-zinc-800/40 rounded-lg overflow-hidden transition-all">
                
                <div className="flex items-center justify-between p-3 hover:bg-zinc-800/50 transition-colors group select-none">
                  <div className="flex items-center gap-3">
                    <span className={`w-2.5 h-2.5 rounded-full ${style.dot}`}></span>
                    <span className="font-mono text-zinc-200 font-bold text-base">ID {member.id}</span>
                    <span className={`text-[10px] uppercase tracking-wider px-2 py-0.5 rounded border font-bold ${style.badge}`}>
                      {member.role}
                    </span>
                  </div>
                  
                  <div className="text-sm font-mono text-zinc-500 group-hover:text-zinc-300 transition-colors">
                    {member.address}
                  </div>
                </div>
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
      </div>
      
      {/* Footer */}
      <div className="bg-[#151618] border-t border-zinc-800 p-4 shrink-0 shadow-[0_-10px_20px_rgba(0,0,0,0.2)] z-10">
        <div className="text-center text-[10px] text-zinc-600 font-mono tracking-wide">
          "Visualize os nós mapeados na sua área de vizinhança na topologia do Anel Chord."
        </div>
      </div>
      
    </div>
  );
}