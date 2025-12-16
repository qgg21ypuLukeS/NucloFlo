import io
from flask import Flask, request, jsonify, send_file, render_template
from Bio.Blast import NCBIWWW

app = Flask(__name__)
app.config['MAX_CONTENT_LENGTH'] = 2 * 1024 * 1024  # 2 MB max upload

# Route to serve HTML page
@app.route("/try")
def try_page():
    return render_template("try.html")

# Route to run remote BLAST
@app.route("/run_blast", methods=["POST"])
def run_blast():
    if 'file' not in request.files:
        return jsonify({"error": "Missing 'file' in request"}), 400
    if 'blastType' not in request.form:
        return jsonify({"error": "Missing 'blastType' in form data"}), 400

    uploaded = request.files['file']
    blast_type = request.form['blastType']

    # Only accept supported types
    if blast_type not in ["blastn", "blastp", "blastx", "tblastn", "tblastx"]:
        return jsonify({"error": f"Unsupported blastType '{blast_type}'"}), 400

    try:
        # Read file content
        sequence_data = uploaded.read().decode("utf-8")

        # Run remote BLAST on NCBI servers
        result_handle = NCBIWWW.qblast(blast_type, "nt", sequence_data)  # 'nt' for nucleotides
        result_xml = result_handle.read()
        result_handle.close()

        # Return result as downloadable XML file
        return send_file(
            io.BytesIO(result_xml.encode("utf-8")),
            as_attachment=True,
            attachment_filename="blast_result.xml",
            mimetype="application/xml"
        )

    except Exception as e:
        return jsonify({"error": "Remote BLAST failed", "details": str(e)}), 500

if __name__ == "__main__":
    app.run(host="127.0.0.1", port=5001, debug=True)
