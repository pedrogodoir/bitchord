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

// Estilização segura para o Tailwind CSS
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

  const fetchData = async () => {
    try {
      const data: NodeInfo = await invoke("get_node_info");
      setNode(data);
    } catch (e) {
      console.error("Erro ao buscar dados do nó:", e);
    }
  };

  useEffect(() => {
    fetchData();
    const interval = setInterval(fetchData, 2000);
    return () => clearInterval(interval);
  }, []);

  const handleUpload = async () => {
    const file = await open({
      multiple: false,
      directory: false,
    });
    
    if (file) {
      console.log("Arquivo selecionado:", file);
      await invoke("upload_file", { file });
    }
  };

  if (!node) return (
    <div className="h-full w-full flex flex-col items-center justify-center bg-[#1e1f22] text-zinc-400 font-mono text-xs">
      <div className="w-6 h-6 border-2 border-blue-500 border-t-transparent rounded-full animate-spin mb-3"></div>
      <span>AGUARDANDO CONTATO COM O CORPO DO NÓ...</span>
    </div>
  );

  // Consolidação dos nós conhecidos para exibir na listagem central
  const knownMembers = [
    {
      role: "Sucessor",
      id: node.successor_id,
      address: node.successor_address,
    }
  ];

  // Adiciona o predecessor se ele existir e for diferente do sucessor
  if (node.predecessor_id !== null && node.predecessor_address !== null) {
    if (node.predecessor_id !== node.successor_id) {
       knownMembers.push({
         role: "Predecessor",
         id: node.predecessor_id,
         address: node.predecessor_address,
       });
    }
  }

  return (
    <div className="h-full w-full flex flex-col bg-[#1e1f22] text-zinc-200 font-sans select-none overflow-hidden text-sm">
      
      {/* Painel do Host (Sua Máquina / Seu Nó) */}
      <div className="p-5 bg-[#232428] border-b border-zinc-800 flex items-center justify-between shrink-0 shadow-sm z-10">
        <div className="flex items-center gap-4">
          <div className="w-12 h-12 bg-gradient-to-b from-emerald-500 to-emerald-600 rounded-md flex items-center justify-center shadow-lg border border-emerald-400/20">
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

      {/* Área da Árvore de Redes */}
      <div className="p-4 bg-[#1e1f22] flex-1 flex flex-col overflow-y-auto">
        
        {/* Cabeçalho da Lista - Com Ícone Wired Network */}
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

        {/* Lista Dinâmica dos Membros Conectados */}
        <div className="pl-6 pr-2 space-y-2 border-l border-zinc-800/60 ml-4 flex-1">
            
          {knownMembers.map((member, index) => {
             const style = roleStyles[member.role] || roleStyles.Sucessor;

             return (
               <div key={index} className="flex items-center justify-between p-3 bg-zinc-800/30 hover:bg-zinc-800/60 border border-zinc-800/50 rounded-lg transition-colors group">
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
             )
          })}

          {/* Estado onde ainda não achou o Predecessor */}
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
      
      {/* Novo Footer Call-to-Action */}
      <div className="bg-[#151618] border-t border-zinc-800 p-4 shrink-0 shadow-[0_-10px_20px_rgba(0,0,0,0.2)] z-10">
        <button 
          onClick={handleUpload}
          className="w-full flex items-center justify-center gap-2 py-3 bg-blue-600 hover:bg-blue-500 active:bg-blue-700 border border-blue-500 rounded-md text-sm font-bold text-white transition-all shadow-md"
        >
          <svg className="w-5 h-5 text-blue-200" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2.5" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12" />
          </svg>
          INJETAR ARQUIVO NO ANEL
        </button>
        <div className="text-center mt-2 text-[10px] text-zinc-600 font-mono tracking-wide">
          Selecione um arquivo para fragmentar e publicar no Chord Network.
        </div>
      </div>
      
    </div>
  );
}