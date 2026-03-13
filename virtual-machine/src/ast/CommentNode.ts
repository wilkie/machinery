import Node from './Node';

class CommentNode extends Node {
  readonly message: string;

  constructor(message: string) {
    super();
    this.message = message;
  }
}

export default CommentNode;
