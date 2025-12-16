
var fileInput = document.getElementById('fileInput');
var blastType = document.getElementById('blastType');
var goButton = document.getElementById('goButton');
var filenameDisplay = document.getElementById('filename');

// Show filename
fileInput.addEventListener('change', function () {
    if (fileInput.files.length > 0) {
        filenameDisplay.textContent = "Selected file: " + fileInput.files[0].name;
    } else {
        filenameDisplay.textContent = "";
    }
});

// Show GO only when BLAST type is selected
blastType.addEventListener('change', function () {
    if (blastType.value !== "") {
        goButton.style.display = "inline-block";
    } else {
        goButton.style.display = "none";
    }
});

// Backend call function (ES5 version)
function uploadAndRunBlast(file, blastType) {
    var form = new FormData();
    form.append('file', file);
    form.append('blastType', blastType);
    form.append('db', 'nt');
    form.append('evalue', '1e-6');
    form.append('outfmt', '5');

    fetch('http://127.0.0.1:5001/run_blast', {
        method: 'POST',
        body: form
    })
    .then(function (resp) {
        if (!resp.ok) {
            return resp.json().then(function(err) {
                alert('Server error: ' + (err.error || resp.statusText));
            });
        }

        return resp.blob();
    })
    .then(function (blob) {
        if (!blob) return;

        var url = URL.createObjectURL(blob);
        var a = document.createElement('a');
        a.href = url;
        a.download = 'blast_result.xml';
        document.body.appendChild(a);
        a.click();
        a.remove();
        URL.revokeObjectURL(url);
    })
    .catch(function (err) {
        console.error(err);
        alert('Network or client error: ' + err.message);
    });
}

// When GO is pressed
goButton.addEventListener('click', function () {
    if (!fileInput.files.length) {
        alert('Please choose a file');
        return;
    }
    if (!blastType.value) {
        alert('Please choose a BLAST type');
        return;
    }
    uploadAndRunBlast(fileInput.files[0], blastType.value);
});
