use sha3::{Digest, Sha3_512};

#[derive(Debug, Clone)]
enum AST {
    Add {
        uuid: String,
        items: Vec<AST>, // two users should be allowed to add elements concurrently without conflict? or maybe a light conflict that you can easily resolve?
    },
    Integer {
        uuid: String,
        value: i64, // e.g. if one user updates this, then this should be fine. but two users updating it should create a conflict
    },
}

#[derive(Debug, Clone)]
pub struct ASTHistoryEntry {
    previous: Vec<String>,
    peer: String, // TODO sign with this peer id
    value: ASTHistoryEntryInner,
}

impl ASTHistoryEntry {
    fn hash(&self) -> String {
        let hasher = Sha3_512::new();
        let hasher = self
            .previous
            .iter()
            .fold(hasher, |hasher, val| hasher.chain_update(val));
        let hasher = hasher
            .chain_update(&self.peer)
            .chain_update(format!("{:?}", self.value));
        let hash = hasher.finalize();
        let hex_hash = base16ct::lower::encode_string(&hash);
        hex_hash
    }
}

#[derive(Debug, Clone)]
enum ASTHistoryEntryInner {
    Initial { ast: AST },
    SetInteger { uuid: String, value: i64 },
    InsertToAdd { uuid: String, ast: AST },
}

fn main() {
    let ast_peer_1 = vec![ASTHistoryEntry {
        peer: "1".to_string(),
        previous: vec![],
        value: ASTHistoryEntryInner::Initial {
            ast: AST::Integer {
                uuid: "test".to_owned(),
                value: 42,
            },
        },
    }];

    let mut ast_peer_2 = ast_peer_1.clone();
    ast_peer_2.push(ASTHistoryEntry {
        peer: "2".to_string(),
        previous: vec![ast_peer_2[0].hash()],
        value: ASTHistoryEntryInner::SetInteger {
            uuid: "test".to_owned(),
            value: 43,
        },
    });
    println!("{:?}", ast_peer_2);

    let mut ast_peer_2_iter = ast_peer_2.iter();
    let Some(ASTHistoryEntry {
        previous,
        peer,
        value: ASTHistoryEntryInner::Initial { ast },
    }) = ast_peer_2_iter.next()
    else {
        panic!()
    };

    // peer to peer is cool
}
